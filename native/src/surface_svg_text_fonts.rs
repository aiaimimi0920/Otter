//! On-demand font adaptation for usvg; propagate resolver errors and check laid-out results for missing glyphs.
use super::fonts::{Face, Fonts, Properties};
use resvg::usvg::{self, fontdb};
use skrifa::{instance::Size, MetadataProvider};
use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
};
// Pin the default family of the currently measured old host, not a cross-machine configuration inferred from navigator.language.
pub(super) const DEFAULT_FAMILY: &str = "Microsoft YaHei";
pub(super) struct TextFonts(Arc<Mutex<State>>);
struct State {
    fonts: Fonts,
    sources: Vec<(Arc<Vec<u8>>, Vec<fontdb::ID>)>,
    selections: HashMap<usvg::Font, fontdb::ID>,
    error: Option<io::Error>,
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Unsupported SVG font mapping or missing glyph",
    )
}
impl TextFonts {
    pub(super) fn new() -> io::Result<Self> {
        Ok(Self(Arc::new(Mutex::new(State {
            fonts: Fonts::new()?,
            sources: Vec::new(),
            selections: HashMap::new(),
            error: None,
        }))))
    }
    pub(super) fn resolver(&self) -> usvg::FontResolver<'static> {
        let select = self.0.clone();
        let fallback = self.0.clone();
        usvg::FontResolver {
            select_font: Box::new(move |font, db| {
                let mut state = select.lock().ok()?;
                if state.error.is_some() {
                    return None;
                }
                match state.select(font, db) {
                    Ok(id) => Some(id),
                    Err(e) => {
                        state.error = Some(e);
                        None
                    }
                }
            }),
            select_fallback: Box::new(move |c, excluded, db| {
                let mut state = fallback.lock().ok()?;
                if state.error.is_some() {
                    return None;
                }
                match state.fallback(c, excluded, db) {
                    Ok(id) => Some(id),
                    Err(e) => {
                        state.error = Some(e);
                        None
                    }
                }
            }),
        }
    }
    pub(super) fn finish(&self, tree: &usvg::Tree) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|_| invalid())?
            .error
            .take()
            .map_or(Ok(()), Err)?;
        validate_layout(tree.root())
    }
}
// Resolver success does not guarantee shaping success: usvg fallback may exit early on glyph-count
// mismatch and retain glyph 0. Inspect actual layout rather than infer text completeness from nontransparent pixels.
// validate_font separately guards font-load failures before layout, independently of surviving-node counts.
// This does not audit content discarded early by convert_span/zero bbox and is not proof of source-text completeness.
fn validate_layout(group: &usvg::Group) -> io::Result<()> {
    for node in group.children() {
        if let usvg::Node::Text(text) = node {
            for span in text.layouted() {
                if span.positioned_glyphs.iter().any(|glyph| glyph.id.0 == 0) {
                    return Err(invalid());
                }
            }
        }
        if let usvg::Node::Group(child) = node {
            validate_layout(child)?;
        }
        let mut result = Ok(());
        node.subroots(|root| {
            if result.is_ok() {
                result = validate_layout(root);
            }
        });
        result?;
    }
    Ok(())
}
impl State {
    fn select(
        &mut self,
        font: &usvg::Font,
        db: &mut Arc<fontdb::Database>,
    ) -> io::Result<fontdb::ID> {
        if let Some(id) = self.selections.get(font) {
            return Ok(*id);
        }
        if self.selections.len() >= 256
            || font.families().len() > 16
            || font.variations().iter().any(|axis| !axis.value.is_finite())
        {
            return Err(invalid());
        }
        let stretch = font.stretch() as u8 + 1;
        let style = match font.style() {
            usvg::FontStyle::Normal => 0,
            usvg::FontStyle::Italic => 2,
            usvg::FontStyle::Oblique => 1,
        };
        let mut selected = None;
        for family in font.families() {
            let name = match family {
                svgtypes::FontFamily::Named(name) => name.as_str(),
                svgtypes::FontFamily::Serif => "SimSun",
                svgtypes::FontFamily::SansSerif => DEFAULT_FAMILY,
                svgtypes::FontFamily::Monospace => "Consolas",
                svgtypes::FontFamily::Cursive => "Comic Sans MS",
                svgtypes::FontFamily::Fantasy => "Impact",
            };
            if let Some(face) = self.fonts.select(name, font.weight(), stretch, style)? {
                selected = Some(face);
                break;
            }
        }
        let face = match selected {
            Some(face) => face,
            None => self
                .fonts
                .select(DEFAULT_FAMILY, font.weight(), stretch, style)?
                .ok_or_else(invalid)?,
        };
        let id = self.install(face, db)?;
        validate_font(db, id, font.variations())?;
        self.selections.insert(font.clone(), id);
        Ok(id)
    }
    fn fallback(
        &mut self,
        c: char,
        excluded: &[fontdb::ID],
        db: &mut Arc<fontdb::Database>,
    ) -> io::Result<fontdb::ID> {
        let base = db
            .face(*excluded.first().ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        let family = base.families.first().ok_or_else(invalid)?.0.clone();
        let properties = Properties {
            weight: base.weight.0,
            stretch: base.stretch as u8,
            style: match base.style {
                fontdb::Style::Normal => 0,
                fontdb::Style::Italic => 2,
                fontdb::Style::Oblique => 1,
            },
        };
        // Currently only a single-character candidate source; no claim of preserving DWrite's whole-range shaping semantics.
        let mapped = self
            .fonts
            .map_text(&c.to_string(), &family, properties, "zh-CN", false)?;
        if mapped.scale != 1.0 || mapped.utf16_len != c.len_utf16() as u32 {
            return Err(invalid());
        }
        let id = self.install(mapped.face.ok_or_else(invalid)?, db)?;
        if excluded.contains(&id) {
            return Err(invalid());
        }
        // Fallback callbacks carry no variations; every design space actually selected during this request
        // must load. Conservative rejection is safer than silent library text loss on unknown spans; counts obey the 256 budget.
        for font in self.selections.keys() {
            validate_font(db, id, font.variations())?;
        }
        Ok(id)
    }
    fn install(&mut self, face: Face, db: &mut Arc<fontdb::Database>) -> io::Result<fontdb::ID> {
        // Never fabricate simulated weight/italic or ignore fallback scale; real outline transforms require separate integration later.
        if face.simulations != 0 {
            return Err(invalid());
        }
        if let Some((_, ids)) = self
            .sources
            .iter()
            .find(|(bytes, _)| Arc::ptr_eq(bytes, &face.bytes))
        {
            return ids
                .iter()
                .copied()
                .find(|id| db.face(*id).is_some_and(|f| f.index == face.index))
                .ok_or_else(invalid);
        }
        let ids = Arc::make_mut(db)
            .load_font_source(fontdb::Source::Binary(face.bytes.clone()))
            .to_vec();
        let id = ids
            .iter()
            .copied()
            .find(|id| db.face(*id).is_some_and(|f| f.index == face.index))
            .ok_or_else(invalid)?;
        self.sources.push((face.bytes, ids));
        Ok(id)
    }
}

// Match pinned usvg 0.48.1 load_font failure conditions using the same-version skrifa parser.
// fontdb FaceInfo existence proves only successful directory parsing, not availability of layout-required metrics.
fn validate_font(
    db: &fontdb::Database,
    id: fontdb::ID,
    variations: &[usvg::FontVariation],
) -> io::Result<()> {
    db.with_face_data(id, |data, index| {
        let font = skrifa::FontRef::from_index(data, index).map_err(|_| invalid())?;
        let location = font.axes().location(
            variations
                .iter()
                .map(|axis| (skrifa::Tag::from_be_bytes(axis.tag), axis.value)),
        );
        let metrics = font.metrics(Size::unscaled(), &location);
        validate_metrics(
            metrics.units_per_em,
            metrics.ascent,
            metrics.descent,
            metrics.x_height,
        )
    })
    .ok_or_else(invalid)?
}
fn validate_metrics(
    units_per_em: u16,
    ascent: f32,
    descent: f32,
    x_height: Option<f32>,
) -> io::Result<()> {
    if !(16..=16384).contains(&units_per_em) || !ascent.is_finite() || !descent.is_finite() {
        return Err(invalid());
    }
    let x_height = x_height
        .filter(|x| *x > 0.0)
        .and_then(|x| u16::try_from(x.round() as i32).ok())
        .filter(|x| *x != 0)
        .or_else(|| {
            u16::try_from(((ascent - descent) * 0.45).round() as i32)
                .ok()
                .filter(|x| *x != 0)
        });
    x_height.ok_or_else(invalid).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_svg_font_metrics_reject_load_failures_before_layout() {
        for units in [0, 15, 16385, u16::MAX] {
            assert!(validate_metrics(units, 800.0, -200.0, Some(400.0)).is_err());
        }
        for (ascent, descent) in [
            (0.0, 0.0),
            (0.0, 20.0),
            (f32::NAN, 0.0),
            (800.0, f32::INFINITY),
        ] {
            assert!(validate_metrics(1000, ascent, descent, None).is_err());
        }
        for height in [None, Some(0.0), Some(-2.0), Some(400.0)] {
            assert!(validate_metrics(1000, 800.0, -200.0, height).is_ok());
        }
        assert!(validate_font(&fontdb::Database::new(), fontdb::ID::dummy(), &[]).is_err());
    }
}
