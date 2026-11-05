//! 解析合同不读取网络/文件，覆盖旧源选择外的拒绝与有界取消。
use super::*;
#[test]
fn inline_data_percent_and_forgiving_base64() {
    for body in ["YQ==", "YQ", "YR", "Y%20Q%3D%3D", "YQ%0A=="] {
        assert_eq!(
            decode(&format!("data:image/png;base64,{body}"), || false).unwrap(),
            b"a"
        );
    }
    assert_eq!(
        decode("data:image/png;charset=utf-8; BASE64,YQ==#ignored", || {
            false
        })
        .unwrap(),
        b"a"
    );
    assert_eq!(
        decode("data:image/png,%89PNG%00+%ZZ?x#discard", || false).unwrap(),
        b"\x89PNG\0+%ZZ?x"
    );
    assert_eq!(
        mime("data:image/PNG;charset=utf-8;base64,YQ==").unwrap(),
        "image/png"
    );
}
#[test]
fn inline_data_rejects_invalid_base64_sources_and_over_budget() {
    for body in ["Y", "YQ=", "Y===", "YQ===", "YQ-_", "YQ%0B=="] {
        assert!(
            decode(&format!("data:image/png;base64,{body}"), || false).is_err(),
            "{body}"
        );
    }
    for source in [
        "data:image/png",
        "DATA:image/png,YQ",
        "blob:any",
        "file:///test.png",
        "https://localhost/test",
    ] {
        assert!(decode(source, || false).is_err(), "{source}");
    }
    assert_eq!(
        decode(&format!("data:image/png,{}", "a".repeat(LIMIT)), || false),
        Err(ResourceError::Limit)
    );
    assert_eq!(
        decode("data:image/png,YQ==", || true),
        Err(ResourceError::Identity)
    );
}
#[test]
fn inline_data_unknown_mime_preserves_bytes_and_svg_is_deferred_to_child() {
    for mime in [
        "image/webp",
        "image/unknown",
        "image/ png",
        "image/png; charset=binary",
    ] {
        assert_eq!(
            decode(&format!("data:{mime};base64,YQ=="), || false).unwrap(),
            b"a"
        );
    }
    for source in [
        "data:image/sv\ng+xml;base64,YQ==",
        "data:image/SVG+XML;charset=utf-8,YQ",
    ] {
        assert!(decode(source, || false).is_ok(), "{source}");
        assert_eq!(mime(source).unwrap(), "image/svg+xml");
    }
    assert!(decode("data:image/png#fragment,YQ", || false).is_err());
    assert_eq!(mime("data:image/pn\ng;base64,YQ==").unwrap(), "image/png");
}
