//! 系统颜色框的局部资源 owner；不访问文档、配置或工作区，调用方负责模态/过期准入。
use crate::stroke_style::Color;
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    UI::Controls::Dialogs::*,
};
pub(super) unsafe fn choose(hwnd: HWND, color: Color) -> Option<Color> {
    let Color(r, g, b) = color;
    let mut custom = [COLORREF(0x00ffffff); 16];
    let mut options = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: hwnd,
        rgbResult: COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        ..Default::default()
    };
    if !ChooseColorW(&mut options).as_bool() {
        let error = CommDlgExtendedError();
        if error != COMMON_DLG_ERRORS(0) {
            eprintln!("Native color dialog failed: {error:?}");
        }
        return None;
    }
    let value = options.rgbResult.0;
    Some(Color(value as u8, (value >> 8) as u8, (value >> 16) as u8))
}
pub(super) fn parse_hex(text: &str) -> Option<Color> {
    if text.len() != 7
        || !text.starts_with('#')
        || !text.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return None;
    }
    let value = u32::from_str_radix(&text[1..], 16).ok()?;
    Some(Color((value >> 16) as u8, (value >> 8) as u8, value as u8))
}
pub(super) fn hex(Color(r, g, b): Color) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn html_rgb_roundtrip_keeps_channel_order_and_rejects_non_rgb_input() {
        assert_eq!(hex(parse_hex("#A10fC3").unwrap()), "#a10fc3");
        for value in [
            "",
            "transparent",
            "#fff",
            "#12345678",
            "#12zz34",
            "＃123456",
        ] {
            assert!(parse_hex(value).is_none());
        }
    }
}
