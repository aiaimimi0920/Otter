//! usvg 的按需字体适配；回传 resolver 错误，并检查已布局结果中的缺字。
use super::fonts::{Face, Fonts, Properties};
use resvg::usvg::{self, fontdb};
use skrifa::{instance::Size, MetadataProvider};
use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
};
// 固定当前已实测旧宿主的默认族；不是依据 navigator.language 推断跨机器配置。
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
// resolver 成功不保证 shaping 成功：usvg 的 fallback 可在 glyph 数量不匹配时
// 提前退出并保留 glyph 0。检查真实布局而不是通过“有非透明像素”推断文字完整。
// 布局前的字体加载失败由 validate_font 单独防守，不依赖 surviving node 数量。
// 这里不审计 convert_span/零 bbox 提前丢弃的内容，不能当作源文字完整性证明。
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
        // 当前只作为单字符候选来源，不宣称保留 DWrite 的整段 shaping 语义。
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
        // fallback 回调没有携带 variations；所有本次实际选择过的设计空间都必须
        // 能加载。保守拒绝比让库在未知 span 上静默丢弃文字安全，数量受 256 预算约束。
        for font in self.selections.keys() {
            validate_font(db, id, font.variations())?;
        }
        Ok(id)
    }
    fn install(&mut self, face: Face, db: &mut Arc<fontdb::Database>) -> io::Result<fontdb::ID> {
        // 不伪造模拟字重/斜体，也不忽略 fallback scale；后续需单独接真实轮廓变换。
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

// 与固定 usvg 0.48.1 的 load_font 可失败条件对齐，使用同版本 skrifa 解析器。
// fontdb FaceInfo 存在只证明目录解析成功，不证明排版所需 metrics 可用。
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
