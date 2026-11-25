//! Limited static-bitmap recognition for ordinary Surface; reject unknown headers without adding browser or script-rendering paths.
fn space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' ')
}
fn token(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(b))
}
/// Extract only the MIME essence; parameters do not affect bitmap decoding. Invalid declarations are unknown types, not byte authorization.
pub fn essence(mime: &str) -> Option<&str> {
    let head = mime
        .trim_matches(space)
        .split(';')
        .next()?
        .trim_end_matches(space);
    let (kind, subtype) = head.split_once('/')?;
    (token(kind.as_bytes()) && token(subtype.as_bytes())).then_some(head)
}
pub fn svg(mime: &str) -> bool {
    // Legacy resources reach the browser through data URLs, whose parsing removes TAB/CR/LF; this must not bypass SVG routing.
    if mime.bytes().any(|b| matches!(b, b'\t' | b'\r' | b'\n')) {
        let normalized: String = mime
            .chars()
            .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
            .collect();
        return essence(&normalized).is_some_and(|v| v.eq_ignore_ascii_case("image/svg+xml"));
    }
    essence(mime).is_some_and(|v| v.eq_ignore_ascii_case("image/svg+xml"))
}
/// Recognize only existing static containers; WIC must still recheck the container, frame count, and pixel limits.
pub fn encoded_mime(bytes: &[u8], declared: &str) -> Option<&'static str> {
    // Chromium's SVG-image branch does not fall back to bitmap decoding for PNG bytes; SVG needs its own safe-rendering owner.
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
