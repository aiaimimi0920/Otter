//! 系统 fallback 的范围结果；保留 UTF-16 长度及缩放，不把它退化成单字符字体查找。
use super::{file, Face, Fonts};
use std::io;
use windows::{core::PCWSTR, Win32::Graphics::DirectWrite::*};
#[path = "surface_svg_text_source.rs"]
mod source;
#[derive(Clone, Copy)]
pub struct Properties {
    pub weight: u16,
    pub stretch: u8,
    pub style: u8,
}
impl Default for Properties {
    fn default() -> Self {
        Self {
            weight: 400,
            stretch: 5,
            style: 0,
        }
    }
}
pub struct Mapped {
    /// 输入开头可使用同一字体的 UTF-16 单元数，不是 UTF-8 字节数或 Rust char 数。
    pub utf16_len: u32,
    pub scale: f32,
    /// None 表示系统没有映射字体，调用方必须显式处理缺字，不能当透明成功。
    pub face: Option<Face>,
}
impl Fonts {
    /// 每次最多 16384 UTF-16 单元，共享 select 的查询/文件预算。locale 由宿主传入。
    pub fn map_text(
        &mut self,
        text: &str,
        family: &str,
        properties: Properties,
        locale: &str,
        rtl: bool,
    ) -> io::Result<Mapped> {
        self.begin_query(
            family,
            properties.weight,
            properties.stretch,
            properties.style,
        )?;
        let (source, length) = source::create(text, locale, rtl)?;
        let name: Vec<u16> = family.encode_utf16().chain(Some(0)).collect();
        let (mut mapped, mut font, mut scale) = (0, None, 0.0);
        unsafe {
            self.fallback
                .MapCharacters(
                    &source,
                    0,
                    length,
                    &self.collection,
                    PCWSTR(name.as_ptr()),
                    DWRITE_FONT_WEIGHT(i32::from(properties.weight)),
                    DWRITE_FONT_STYLE(i32::from(properties.style)),
                    DWRITE_FONT_STRETCH(i32::from(properties.stretch)),
                    &mut mapped,
                    &mut font,
                    &mut scale,
                )
                .map_err(io::Error::other)?;
        }
        validate_mapping(text, mapped, scale)?;
        // fallback 的文件来源与显式 family 完全复用同一准入门和缓存，不旁路读取。
        let face = font.map(|font| self.load_font(font)).transpose()?;
        Ok(Mapped {
            utf16_len: mapped,
            scale,
            face,
        })
    }
}
fn validate_mapping(text: &str, mapped: u32, scale: f32) -> io::Result<()> {
    if mapped == 0 || !scale.is_finite() || scale <= 0.0 {
        return Err(file::invalid());
    }
    let mut units = 0;
    for c in text.chars() {
        units += c.len_utf16() as u32;
        if units == mapped {
            return Ok(());
        }
        if units > mapped {
            break;
        }
    }
    Err(file::invalid())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_font_mapping_rejects_zero_split_surrogate_and_nonfinite_scale() {
        assert!(validate_mapping("😀a", 2, 0.8).is_ok());
        assert!(validate_mapping("😀a", 3, 1.0).is_ok());
        for mapped in [0, 1, 4, u32::MAX] {
            assert!(validate_mapping("😀a", mapped, 1.).is_err());
        }
        for scale in [0., -1., f32::NAN, f32::INFINITY] {
            assert!(validate_mapping("a", 1, scale).is_err());
        }
    }
}
