//! 普通 Surface 的有限静态位图识别；未知头拒绝，不新增浏览器或脚本渲染路径。
fn space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' ')
}
fn token(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(b))
}
/// 只提取 MIME essence，参数不影响位图解码。无效声明作为未知类型，不作为字节授权。
pub(crate) fn essence(mime: &str) -> Option<&str> {
    let head = mime
        .trim_matches(space)
        .split(';')
        .next()?
        .trim_end_matches(space);
    let (kind, subtype) = head.split_once('/')?;
    (token(kind.as_bytes()) && token(subtype.as_bytes())).then_some(head)
}
pub(crate) fn svg(mime: &str) -> bool {
    // 旧资源由 data URL 交给浏览器，URL 解析会去掉 TAB/CR/LF；不能绕过 SVG 分流。
    if mime.bytes().any(|b| matches!(b, b'\t' | b'\r' | b'\n')) {
        let normalized: String = mime
            .chars()
            .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
            .collect();
        return essence(&normalized).is_some_and(|v| v.eq_ignore_ascii_case("image/svg+xml"));
    }
    essence(mime).is_some_and(|v| v.eq_ignore_ascii_case("image/svg+xml"))
}
/// 仅识别现有静态容器；WIC 仍须再次核对容器、帧数与像素上限。
pub(crate) fn encoded_mime(bytes: &[u8], declared: &str) -> Option<&'static str> {
    // Chromium 的 SVG 图片分支不把 PNG 字节回退为位图；SVG 另需独立安全渲染 owner。
    if svg(declared) {
        return None;
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else if bytes.starts_with(b"II\x2a\0") || bytes.starts_with(b"MM\0\x2a") {
        Some("image/tiff")
    } else if bytes.starts_with(&[0, 0, 1, 0]) || bytes.starts_with(&[0, 0, 2, 0]) {
        Some("image/x-icon")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_format_headers_are_only_bounded_candidates_for_wic() {
        for (bytes, mime) in [
            (b"\x89PNG\r\n\x1a\n".as_slice(), "image/png"),
            (b"\xff\xd8\xff", "image/jpeg"),
            (b"GIF87a", "image/gif"),
            (b"GIF89a", "image/gif"),
            (b"BM", "image/bmp"),
            (b"RIFFxxxxWEBP", "image/webp"),
            (b"II\x2a\0", "image/tiff"),
            (b"MM\0\x2a", "image/tiff"),
            (b"\0\0\x01\0", "image/x-icon"),
            (b"\0\0\x02\0", "image/x-icon"),
        ] {
            assert_eq!(encoded_mime(bytes, "image/unknown"), Some(mime));
            assert_eq!(encoded_mime(bytes, "image/sv\ng+xml; charset=utf-8"), None);
        }
        for bytes in [b"".as_slice(), b"<svg/>", b"<html/>", b"%PDF"] {
            assert_eq!(encoded_mime(bytes, "image/png"), None);
        }
    }
    #[test]
    fn surface_format_mime_essence_does_not_treat_invalid_token_as_svg() {
        assert_eq!(essence(" IMAGE/PNG ; charset=\"utf-8\""), Some("IMAGE/PNG"));
        assert_eq!(essence("image/ png"), None);
        assert_eq!(essence("image/png\u{b}"), None);
        assert!(svg("image/sv\tg+xml"));
        assert!(!svg("image/svg+xml\u{b}"));
    }
}
