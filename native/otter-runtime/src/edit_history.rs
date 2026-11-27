//! Bounded operation history transfers Stroke ownership for undo/redo without copying images or point arrays.
use crate::editor::Stroke;
use crate::stroke_style::StrokeStyle;
use std::collections::VecDeque;
#[path = "edit_history_source.rs"]
pub mod source;

const LIMIT: usize = 50;

pub struct Document {
    pub elements: Vec<Stroke>,
    pub serial: u32,
    pub raster: Option<crate::erase_layers::Layers>,
    pub crop: Option<crate::editor::CropView>,
    pub logical_size: Option<crate::crop_geometry::Size>,
    pub mirror: crate::editor::MirrorState,
    pub spotlight: Option<u8>,
}

/// Mutable target borrow for the same document, owning no additional history/pixels.
pub struct Target<'a> {
    pub elements: &'a mut Vec<Stroke>,
    pub serial_count: &'a mut u32,
    pub raster: &'a mut Option<crate::erase_layers::Layers>,
    pub crop: &'a mut Option<crate::editor::CropView>,
    pub logical_size: &'a mut Option<crate::crop_geometry::Size>,
    pub mirror: &'a mut crate::editor::MirrorState,
    pub pending_pin: &'a mut Option<crate::editor::PinValue>,
    pub spotlight: &'a mut Option<u8>,
}

pub struct Annotations {
    pub elements: Vec<Stroke>,
    pub serial: u32,
}

pub struct Entry {
    pin: Option<Box<crate::editor::PinChange>>,
    annotations: Option<Box<Annotations>>,
    document: Option<Box<Document>>,
    index: usize,
    occupied: bool,
    stored: Option<Stroke>,
    style: Option<StrokeStyle>,
    rotation: Option<f32>,
    serial: Option<u32>,
    raster: Option<crate::erase_layers::Layers>,
    batch: Option<Vec<(usize, Option<Stroke>)>>,
    replacements: Option<Vec<(usize, Stroke)>>,
    styles: Option<Vec<(usize, StrokeStyle)>>,
    crop: Option<Option<crate::editor::CropView>>,
    mirror: Option<crate::editor::MirrorState>,
    spotlight: Option<u8>,
}

impl Entry {
    pub fn spotlight(percent: u8) -> Self {
        Self {
            spotlight: Some(percent),
            ..Self::added(0)
        }
    }
    pub fn pin(change: crate::editor::PinChange) -> Self {
        Self {
            pin: Some(Box::new(change)),
            ..Self::added(0)
        }
    }
    pub fn annotations(previous: Annotations) -> Self {
        Self {
            annotations: Some(Box::new(previous)),
            ..Self::added(0)
        }
    }
    pub fn document(previous: Document) -> Self {
        Self {
            document: Some(Box::new(previous)),
            ..Self::added(0)
        }
    }
    pub fn with_crop(mut self, previous: Option<crate::editor::CropView>) -> Self {
        self.crop = Some(previous);
        self
    }
    pub fn with_mirror(mut self, previous: crate::editor::MirrorState) -> Self {
        self.mirror = Some(previous);
        self
    }
    pub fn cropped(previous: Option<crate::editor::CropView>) -> Self {
        Self {
            crop: Some(previous),
            ..Self::added(0)
        }
    }
    pub fn added(index: usize) -> Self {
        Self {
            index,
            pin: None,
            occupied: true,
            stored: None,
            style: None,
            rotation: None,
            serial: None,
            raster: None,
            batch: None,
            replacements: None,
            styles: None,
            crop: None,
            document: None,
            annotations: None,
            mirror: None,
            spotlight: None,
        }
    }
    pub fn removed(index: usize, stroke: Stroke) -> Self {
        Self {
            occupied: false,
            stored: Some(stroke),
            ..Self::added(index)
        }
    }
    pub fn styled(index: usize, style: StrokeStyle) -> Self {
        Self {
            style: Some(style),
            ..Self::added(index)
        }
    }
    pub fn styled_many(styles: Vec<(usize, StrokeStyle)>) -> Self {
        Self {
            styles: Some(styles),
            ..Self::added(0)
        }
    }
    pub fn replaced(index: usize, stroke: Stroke) -> Self {
        Self {
            stored: Some(stroke),
            ..Self::added(index)
        }
    }
    fn retained_points(&self) -> usize {
        self.annotations.as_ref().map_or(0, |annotations| {
            annotations
                .elements
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
        }) + self.document.as_ref().map_or(0, |document| {
            document
                .elements
                .iter()
                .map(|stroke| stroke.points.len())
                .sum::<usize>()
        }) + self.stored.as_ref().map_or(0, |s| s.points.len())
            + self.replacements.as_ref().map_or(0, |entries| {
                entries
                    .iter()
                    .map(|(_, stroke)| stroke.points.len())
                    .sum::<usize>()
            })
            + self.batch.as_ref().map_or(0, |entries| {
                entries
                    .iter()
                    .map(|(_, stroke)| stroke.as_ref().map_or(0, |s| s.points.len()))
                    .sum::<usize>()
            })
    }
    pub fn rotated(index: usize, angle: f32) -> Self {
        Self {
            rotation: Some(angle),
            ..Self::added(index)
        }
    }
    pub fn serial_added(index: usize, previous: u32) -> Self {
        Self {
            serial: Some(previous),
            ..Self::added(index)
        }
    }
    pub fn raster(previous: crate::erase_layers::Layers) -> Self {
        Self {
            raster: Some(previous),
            ..Self::added(0)
        }
    }
    pub fn rasterized(
        previous: crate::erase_layers::Layers,
        removed: Vec<(usize, Option<Stroke>)>,
    ) -> Self {
        Self {
            batch: Some(removed),
            occupied: false,
            ..Self::raster(previous)
        }
    }
    pub fn removed_many(removed: Vec<(usize, Option<Stroke>)>) -> Self {
        Self {
            batch: Some(removed),
            occupied: false,
            ..Self::added(0)
        }
    }
    pub fn added_many(indices: std::ops::Range<usize>) -> Self {
        Self {
            batch: Some(indices.map(|index| (index, None)).collect()),
            ..Self::added(0)
        }
    }
    pub fn replaced_many(previous: Vec<(usize, Stroke)>) -> Self {
        Self {
            replacements: Some(previous),
            ..Self::added(0)
        }
    }
    pub fn raster_refreshed(
        previous: crate::erase_layers::Layers,
        replacements: Vec<(usize, Stroke)>,
        removed: Vec<(usize, Option<Stroke>)>,
    ) -> Self {
        Self {
            replacements: Some(replacements),
            batch: (!removed.is_empty()).then_some(removed),
            occupied: false,
            ..Self::raster(previous)
        }
    }
    fn swap_replacements(&mut self, elements: &mut [Stroke]) {
        if let Some(entries) = &mut self.replacements {
            for (index, stored) in entries {
                std::mem::swap(stored, &mut elements[*index]);
            }
        }
    }
    fn toggle(&mut self, target: Target<'_>) {
        let Target {
            elements,
            serial_count,
            raster,
            crop,
            logical_size,
            mirror,
            pending_pin,
            spotlight,
        } = target;
        if let Some(previous) = &mut self.spotlight {
            *previous = spotlight.replace(*previous).unwrap_or(60);
            return;
        }
        if let Some(change) = &mut self.pin {
            *pending_pin = Some(change.toggle());
            return;
        }
        if let Some(annotations) = &mut self.annotations {
            std::mem::swap(&mut annotations.elements, elements);
            std::mem::swap(&mut annotations.serial, serial_count);
            return;
        }
        if let Some(document) = &mut self.document {
            std::mem::swap(&mut document.elements, elements);
            std::mem::swap(&mut document.serial, serial_count);
            std::mem::swap(&mut document.raster, raster);
            std::mem::swap(&mut document.crop, crop);
            std::mem::swap(&mut document.logical_size, logical_size);
            std::mem::swap(&mut document.mirror, mirror);
            std::mem::swap(&mut document.spotlight, spotlight);
            return;
        }
        if let Some(previous) = &mut self.mirror {
            std::mem::swap(previous, mirror);
        }
        if let Some(previous) = &mut self.crop {
            std::mem::swap(previous, crop);
            if self.raster.is_none() {
                return;
            }
        }
        if let Some(styles) = &mut self.styles {
            for (index, style) in styles {
                *style = elements[*index].apply_style(*style);
            }
            return;
        }
        // History owns admitted snapshots: undo/redo must not allocate pixels or fail halfway.
        let raster_change = self.raster.is_some();
        if raster_change {
            std::mem::swap(&mut self.raster, raster);
        }
        if self.batch.is_none() || !self.occupied {
            // Replacement indices refer to the post-removal collection: undo restores effects before reinserting vectors.
            self.swap_replacements(elements);
        }
        if self.replacements.is_some() && self.batch.is_none() {
            return;
        }
        if let Some(entries) = &mut self.batch {
            if self.occupied {
                for (index, stored) in entries.iter_mut().rev() {
                    *stored = Some(elements.remove(*index));
                }
            } else {
                for (index, stored) in entries {
                    elements.insert(*index, stored.take().unwrap());
                }
            }
            if self.occupied {
                // Redo removes vectors first, then swaps remaining effects.
                self.swap_replacements(elements);
            }
            self.occupied = !self.occupied;
            return;
        }
        if raster_change {
            return;
        }
        if let Some(previous) = self.serial {
            self.serial = Some(std::mem::replace(serial_count, previous));
        }
        if let Some(angle) = self.rotation {
            self.rotation = Some(std::mem::replace(&mut elements[self.index].rotation, angle));
            return;
        }
        if let Some(style) = self.style {
            self.style = Some(elements[self.index].apply_style(style));
            return;
        }
        match (self.occupied, self.stored.take()) {
            (true, None) => {
                self.stored = Some(elements.remove(self.index));
                self.occupied = false;
            }
            (false, Some(stroke)) => {
                elements.insert(self.index, stroke);
                self.occupied = true;
            }
            (true, Some(stroke)) => {
                self.stored = Some(std::mem::replace(&mut elements[self.index], stroke));
            }
            (false, None) => unreachable!("history entry has neither document nor stored content"),
        }
    }
}

#[derive(Default)]
pub struct History {
    entries: VecDeque<Entry>,
    cursor: usize,
}
impl History {
    pub fn availability(&self) -> (bool, bool) {
        (self.cursor > 0, self.cursor < self.entries.len())
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Return the actual number of points released when discarding redo branches/oldest records. History eviction never loses document points.
    pub fn record(&mut self, entry: Entry) -> usize {
        let mut released = self
            .entries
            .drain(self.cursor..)
            .map(|e| e.retained_points())
            .sum();
        self.entries.push_back(entry);
        if self.entries.len() > LIMIT {
            released += self.entries.pop_front().unwrap().retained_points();
        }
        self.cursor = self.entries.len();
        released
    }
    pub fn undo(&mut self, target: Target<'_>) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.entries[self.cursor].toggle(target);
        true
    }
    pub fn redo(&mut self, target: Target<'_>) -> bool {
        if self.cursor == self.entries.len() {
            return false;
        }
        self.entries[self.cursor].toggle(target);
        self.cursor += 1;
        true
    }
}
