//! Window-private DWrite/D2D geometry and style caches for committed annotations; released with Target and pruned immediately to active annotations.
use crate::{
    editor::{Stroke, Tool},
    stroke_style::DashPattern,
    text_annotation::TextAnnotation,
    viewport::{Point, Viewport},
};
use std::collections::{HashSet, VecDeque};
use windows::{
    core::Result,
    Win32::Graphics::{Direct2D::*, DirectWrite::IDWriteTextLayout},
};

const MAX_TEXT_ENTRIES: usize = 64;
const MAX_PATH_ENTRIES: usize = 128;
const MAX_STYLE_ENTRIES: usize = 32;

struct TextEntry {
    id: String,
    text: TextAnnotation,
    box_key: [u32; 3],
    layout: IDWriteTextLayout,
}

#[derive(Clone)]
enum PathValue {
    Single(ID2D1PathGeometry),
    Arrow(Option<(ID2D1PathGeometry, ID2D1PathGeometry)>),
    Bands(Vec<(ID2D1PathGeometry, f32)>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Single,
    Arrow,
    Bands,
}

struct PathEntry {
    id: String,
    hash: u64,
    kind: PathKind,
    value: PathValue,
}

struct StyleEntry {
    key: (u8, DashPattern, u32),
    style: Option<ID2D1StrokeStyle>,
}

#[derive(Default)]
pub(in crate::desktop) struct Cache {
    texts: VecDeque<TextEntry>,
    paths: VecDeque<PathEntry>,
    styles: VecDeque<StyleEntry>,
    #[cfg(test)]
    pub(in crate::desktop) text_builds: usize,
    #[cfg(test)]
    pub(in crate::desktop) path_builds: usize,
}

struct Restore<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Restore<F> {
    fn drop(&mut self) {
        if let Some(restore) = self.0.take() {
            restore();
        }
    }
}

pub(in crate::desktop) unsafe fn with_origin<R>(
    target: &ID2D1RenderTarget,
    origin: Point,
    draw: impl FnOnce(&ID2D1RenderTarget) -> R,
) -> R {
    let mut original = Default::default();
    let shifted = origin.x != 0.0 || origin.y != 0.0;
    if shifted {
        target.GetTransform(&mut original);
        let mut next = original;
        next.M31 = origin.x * original.M11 + origin.y * original.M21 + original.M31;
        next.M32 = origin.x * original.M12 + origin.y * original.M22 + original.M32;
        target.SetTransform(&next);
    }
    let _guard = Restore(shifted.then(|| {
        move || {
            target.SetTransform(&original);
        }
    }));
    draw(target)
}

impl Cache {
    #[cfg(test)]
    pub(in crate::desktop) fn entry_counts(&self) -> [usize; 3] {
        [self.texts.len(), self.paths.len(), self.styles.len()]
    }

    pub(in crate::desktop) fn retain_active(&mut self, active: &HashSet<&str>) {
        self.texts
            .retain(|entry| active.contains(entry.id.as_str()));
        self.paths
            .retain(|entry| active.contains(entry.id.as_str()));
    }

    pub(in crate::desktop) fn text_layout(
        &mut self,
        cacheable: bool,
        id: &str,
        text: &TextAnnotation,
        scale: f32,
        width: f32,
        height: f32,
        build: impl FnOnce() -> Result<IDWriteTextLayout>,
    ) -> Result<IDWriteTextLayout> {
        if !cacheable {
            return build();
        }
        let box_key = [scale.to_bits(), width.to_bits(), height.to_bits()];
        if let Some(entry) = self
            .texts
            .iter()
            .find(|e| e.id == id && e.box_key == box_key && &e.text == text)
        {
            return Ok(entry.layout.clone());
        }
        let layout = build()?;
        #[cfg(test)]
        {
            self.text_builds += 1;
        }
        self.texts.retain(|e| e.id != id);
        // Documents redraw in a fixed order; draw transiently at capacity so a scan cannot evict every still-active cache entry.
        if self.texts.len() >= MAX_TEXT_ENTRIES {
            return Ok(layout);
        }
        self.texts.push_back(TextEntry {
            id: id.to_owned(),
            text: text.clone(),
            box_key,
            layout: layout.clone(),
        });
        Ok(layout)
    }

    pub(in crate::desktop) fn single_path(
        &mut self,
        cacheable: bool,
        stroke: &Stroke,
        scale: f32,
        build: impl FnOnce(Viewport) -> Result<ID2D1PathGeometry>,
    ) -> Result<ID2D1PathGeometry> {
        let local = Viewport {
            origin: Point::default(),
            scale,
        };
        if !cacheable {
            return build(local);
        }
        match self.cached_path(stroke, scale, PathKind::Single, || {
            build(local).map(PathValue::Single)
        })? {
            PathValue::Single(path) => Ok(path),
            _ => unreachable!("path kind is part of the cache key"),
        }
    }

    pub(in crate::desktop) fn arrow_paths(
        &mut self,
        cacheable: bool,
        stroke: &Stroke,
        scale: f32,
        build: impl FnOnce(Viewport) -> Result<Option<(ID2D1PathGeometry, ID2D1PathGeometry)>>,
    ) -> Result<Option<(ID2D1PathGeometry, ID2D1PathGeometry)>> {
        let local = Viewport {
            origin: Point::default(),
            scale,
        };
        if !cacheable {
            return build(local);
        }
        match self.cached_path(stroke, scale, PathKind::Arrow, || {
            build(local).map(PathValue::Arrow)
        })? {
            PathValue::Arrow(paths) => Ok(paths),
            _ => unreachable!("path kind is part of the cache key"),
        }
    }

    pub(in crate::desktop) fn paintbrush_bands(
        &mut self,
        cacheable: bool,
        stroke: &Stroke,
        scale: f32,
        build: impl FnOnce(Viewport) -> Result<Vec<(ID2D1PathGeometry, f32)>>,
    ) -> Result<Vec<(ID2D1PathGeometry, f32)>> {
        let local = Viewport {
            origin: Point::default(),
            scale,
        };
        if !cacheable {
            return build(local);
        }
        match self.cached_path(stroke, scale, PathKind::Bands, || {
            build(local).map(PathValue::Bands)
        })? {
            PathValue::Bands(bands) => Ok(bands),
            _ => unreachable!("path kind is part of the cache key"),
        }
    }

    pub(in crate::desktop) fn stroke_style(
        &mut self,
        kind: u8,
        pattern: DashPattern,
        width: f32,
        build: impl FnOnce() -> Result<Option<ID2D1StrokeStyle>>,
    ) -> Result<Option<ID2D1StrokeStyle>> {
        let key = (kind, pattern, width.to_bits());
        if let Some(index) = self.styles.iter().position(|e| e.key == key) {
            let entry = self.styles.remove(index).unwrap();
            let style = entry.style.clone();
            self.styles.push_back(entry);
            return Ok(style);
        }
        let style = build()?;
        while self.styles.len() >= MAX_STYLE_ENTRIES {
            self.styles.pop_front();
        }
        self.styles.push_back(StyleEntry {
            key,
            style: style.clone(),
        });
        Ok(style)
    }

    fn cached_path(
        &mut self,
        stroke: &Stroke,
        scale: f32,
        kind: PathKind,
        build: impl FnOnce() -> Result<PathValue>,
    ) -> Result<PathValue> {
        let id = stroke.annotation_id();
        let hash = geometry_hash(stroke, scale);
        if let Some(entry) = self
            .paths
            .iter()
            .find(|e| e.id == id && e.hash == hash && e.kind == kind)
        {
            return Ok(entry.value.clone());
        }
        let value = build()?;
        #[cfg(test)]
        {
            self.path_builds += 1;
        }
        // Arrow(None) for a degenerate arrow and the ordinary curve fallback are distinct geometry types for the same annotation.
        self.paths.retain(|e| e.id != id || e.kind != kind);
        if self.paths.len() >= MAX_PATH_ENTRIES {
            return Ok(value);
        }
        self.paths.push_back(PathEntry {
            id: id.to_owned(),
            hash,
            kind,
            value: value.clone(),
        });
        Ok(value)
    }
}

fn geometry_hash(stroke: &Stroke, scale: f32) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |bytes: &[u8]| {
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    mix(&[stroke.tool as u8, stroke.sides, u8::from(stroke.arrow)]);
    mix(&scale.to_bits().to_le_bytes());
    mix(&stroke.width.to_bits().to_le_bytes());
    mix(&stroke.corner_radius.to_bits().to_le_bytes());
    if stroke.tool == Tool::Paintbrush {
        mix(&(stroke.opacity * stroke.paint_alpha.stroke())
            .to_bits()
            .to_le_bytes());
        mix(&(stroke.brush_weights.len() as u64).to_le_bytes());
        for &weight in &stroke.brush_weights {
            mix(&weight.to_bits().to_le_bytes());
        }
    }
    mix(&(stroke.points.len() as u64).to_le_bytes());
    for point in &stroke.points {
        mix(&point.x.to_bits().to_le_bytes());
        mix(&point.y.to_bits().to_le_bytes());
    }
    h
}
