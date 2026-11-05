//! 真实系统映射基线，不把字体映射成功当成复杂文字 shaping 或 SVG 绘制通过。
use super::*;
pub(super) fn run() -> io::Result<Vec<serde_json::Value>> {
    let mut records = Vec::new();
    for (text, locale, rtl) in [
        ("Hello", "en-US", false),
        ("中文测试", "zh-CN", false),
        ("A中文😀", "zh-CN", false),
        ("😀", "en-US", false),
        ("العربية", "ar-SA", true),
        ("नमस्ते", "hi-IN", false),
        ("👩‍💻", "en-US", false),
        ("\u{10ffff}", "en-US", false),
    ] {
        let mut fonts = Fonts::new()?;
        let mut rest = text;
        let mut chunks = Vec::new();
        while !rest.is_empty() {
            let mapped = fonts.map_text(rest, "Arial", Properties::default(), locale, rtl)?;
            let mut units = 0;
            let mut byte_end = 0;
            for c in rest.chars() {
                units += c.len_utf16() as u32;
                byte_end += c.len_utf8();
                if units == mapped.utf16_len {
                    break;
                }
            }
            assert_eq!(units, mapped.utf16_len);
            let missing = mapped.face.is_none();
            assert_eq!(missing, text == "\u{10ffff}", "{text:?}");
            let info = mapped.face.as_ref().map(|face| {
                let path = fonts.files.iter().find(|(_, bytes)| Arc::ptr_eq(bytes, &face.bytes)).unwrap().0;
                serde_json::json!({"file":path.file_name().unwrap().to_string_lossy(), "index":face.index,"simulations":face.simulations})
            });
            chunks.push(serde_json::json!({"utf16Length":mapped.utf16_len,"scale":mapped.scale,"face":info}));
            rest = &rest[byte_end..];
        }
        assert!(!chunks.is_empty());
        if text == "Hello" {
            assert_eq!(chunks[0]["utf16Length"], 5);
        }
        if text == "A中文😀" {
            assert_eq!(chunks[0]["utf16Length"], 1);
        }
        records.push(serde_json::json!({"text":text,"locale":locale,"chunks":chunks}));
    }
    let mut fonts = Fonts::new()?;
    let arial = fonts.select("Arial", 400, 5, 0)?.unwrap();
    let mapped = fonts.map_text("Hi", "Arial", Properties::default(), "en-US", false)?;
    assert!(Arc::ptr_eq(&arial.bytes, &mapped.face.unwrap().bytes));
    assert_eq!(fonts.files.len(), 1);
    fonts.queries = MAX_QUERIES;
    assert!(fonts
        .map_text("中", "Arial", Properties::default(), "zh-CN", false)
        .is_err());
    fonts.queries = 0;
    fonts.bytes = MAX_BYTES;
    assert!(fonts
        .map_text("中", "Arial", Properties::default(), "zh-CN", false)
        .is_err());
    Ok(records)
}
