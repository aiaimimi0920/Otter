//! 同步 DirectWrite 分析的有界 UTF-16 backing store；COM 指针不越过此次调用生命周期。
use std::io;
use windows::{
    core::{implement, OutRef, Result},
    Win32::{
        Foundation::{E_INVALIDARG, E_POINTER},
        Graphics::DirectWrite::*,
    },
};
pub(super) const MAX_UNITS: usize = 16384;
#[implement(IDWriteTextAnalysisSource)]
struct Source {
    text: Vec<u16>,
    locale: Vec<u16>,
    rtl: bool,
}
pub(super) fn create(
    text: &str,
    locale: &str,
    rtl: bool,
) -> io::Result<(IDWriteTextAnalysisSource, u32)> {
    let count = text.encode_utf16().count();
    if count == 0
        || count > MAX_UNITS
        || locale.is_empty()
        || locale.len() > 84
        || !locale
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(super::file::invalid());
    }
    Ok((
        Source {
            text: text.encode_utf16().collect(),
            locale: locale.encode_utf16().chain(Some(0)).collect(),
            rtl,
        }
        .into(),
        count as u32,
    ))
}
impl IDWriteTextAnalysisSource_Impl for Source_Impl {
    fn GetTextAtPosition(
        &self,
        position: u32,
        text: *mut *mut u16,
        length: *mut u32,
    ) -> Result<()> {
        if text.is_null() || length.is_null() {
            return Err(E_POINTER.into());
        }
        let position = position as usize;
        unsafe {
            *text = std::ptr::null_mut();
            *length = 0;
            if position < self.text.len() {
                *text = self.text.as_ptr().add(position).cast_mut();
                *length = (self.text.len() - position) as u32;
            }
        }
        Ok(())
    }
    fn GetTextBeforePosition(
        &self,
        position: u32,
        text: *mut *mut u16,
        length: *mut u32,
    ) -> Result<()> {
        if text.is_null() || length.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *text = std::ptr::null_mut();
            *length = 0;
            if position > 0 && position as usize <= self.text.len() {
                *text = self.text.as_ptr().cast_mut();
                *length = position;
            }
        }
        Ok(())
    }
    fn GetParagraphReadingDirection(&self) -> DWRITE_READING_DIRECTION {
        if self.rtl {
            DWRITE_READING_DIRECTION_RIGHT_TO_LEFT
        } else {
            DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
        }
    }
    fn GetLocaleName(&self, position: u32, length: *mut u32, name: *mut *mut u16) -> Result<()> {
        if length.is_null() || name.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *length = 0;
            *name = std::ptr::null_mut();
        }
        if position as usize >= self.text.len() {
            return Err(E_INVALIDARG.into());
        }
        unsafe {
            *length = self.text.len() as u32 - position;
            *name = self.locale.as_ptr().cast_mut();
        }
        Ok(())
    }
    fn GetNumberSubstitution(
        &self,
        position: u32,
        length: *mut u32,
        substitution: OutRef<'_, IDWriteNumberSubstitution>,
    ) -> Result<()> {
        if length.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *length = 0;
        }
        substitution.write(None)?;
        if position as usize >= self.text.len() {
            return Err(E_INVALIDARG.into());
        }
        // 只提供原始字符；不隐式改变数字。完整数字替换属于后续排版合同。
        unsafe {
            *length = self.text.len() as u32 - position;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_font_analysis_source_owns_utf16_ranges_and_locale() {
        let (source, count) = create("A😀中", "zh-CN", true).unwrap();
        assert_eq!(count, 4);
        unsafe {
            let (mut text, mut length) = (std::ptr::null_mut(), 0);
            source.GetTextAtPosition(1, &mut text, &mut length).unwrap();
            assert_eq!(length, 3);
            assert_eq!(
                std::slice::from_raw_parts(text, length as usize),
                &[0xd83d, 0xde00, 0x4e2d]
            );
            source
                .GetTextBeforePosition(3, &mut text, &mut length)
                .unwrap();
            assert_eq!(
                std::slice::from_raw_parts(text, length as usize),
                &[65, 0xd83d, 0xde00]
            );
            for position in [4, u32::MAX] {
                source
                    .GetTextAtPosition(position, &mut text, &mut length)
                    .unwrap();
                assert!(text.is_null());
                assert_eq!(length, 0);
            }
            for position in [0, u32::MAX] {
                source
                    .GetTextBeforePosition(position, &mut text, &mut length)
                    .unwrap();
                assert!(text.is_null());
                assert_eq!(length, 0);
            }
            source.GetLocaleName(1, &mut length, &mut text).unwrap();
            assert_eq!(length, 3);
            assert_eq!(
                std::slice::from_raw_parts(text, 6),
                &[122, 104, 45, 67, 78, 0]
            );
            assert_eq!(
                source.GetParagraphReadingDirection(),
                DWRITE_READING_DIRECTION_RIGHT_TO_LEFT
            );
            assert!(source
                .GetTextAtPosition(0, std::ptr::null_mut(), &mut length)
                .is_err());
            assert!(source.GetLocaleName(4, &mut length, &mut text).is_err());
        }
        for (text, locale) in [
            ("", "en-US"),
            ("a", ""),
            ("a", "en\0US"),
            ("a", "file:///x"),
        ] {
            assert!(create(text, locale, false).is_err());
        }
        assert!(create(&"a".repeat(MAX_UNITS + 1), "en-US", false).is_err());
        assert!(create(&"😀".repeat(MAX_UNITS / 2 + 1), "en-US", false).is_err());
    }
}
