use super::*;
#[test]
fn historical_partial_and_main_hdr_metadata_preserve_known_values_without_inference() {
    let empty = decode("{}").unwrap();
    assert_eq!(empty.dynamic_range, None);
    assert_eq!(empty.bit_depth, None);
    let metadata = decode(r#"{"kind":"region","sourceRect":{"x":-1920,"y":-100,"w":400,"h":300},"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq","captureBackend":"wgc","downgradedFromHdr":false,"futureExtension":{"a":1}}"#).unwrap();
    assert_eq!(metadata.kind, Some(Kind::Region));
    assert_eq!(metadata.dynamic_range, Some(Range::Hdr));
    assert_eq!(metadata.source_rect.unwrap().x, -1920.0);
    assert!(matches!(
        metadata.capture_backend,
        Some(Cow::Borrowed("wgc"))
    ));
    let long = decode(r#"{"kind":"long","scrollAxis":"horizontal","dynamicRange":"sdr","bitDepth":8,"colorSpace":"srgb","downgradedFromHdr":true}"#).unwrap();
    assert_eq!(long.scroll_axis, Some(Axis::Horizontal));
    assert_eq!(long.downgraded_from_hdr, Some(true));
}
#[test]
fn malformed_contradictory_and_unbounded_metadata_are_rejected() {
    for raw in [
        "[]",
        "null",
        r#"{"dynamicRange":"future"}"#,
        r#"{"bitDepth":12}"#,
        r#"{"dynamicRange":"hdr","bitDepth":8}"#,
        r#"{"dynamicRange":"sdr","colorSpace":"bt2020-pq"}"#,
        r#"{"dynamicRange":"hdr","downgradedFromHdr":true}"#,
        r#"{"captureBackend":"bad\u0000backend"}"#,
        r#"{"sourceRect":{"x":0,"y":0,"w":-1,"h":20}}"#,
    ] {
        assert!(decode(raw).is_err(), "{raw}");
    }
    assert!(decode(&format!(r#"{{"captureBackend":"{}"}}"#, "x".repeat(257))).is_err());
    assert!(matches!(
        decode(&" ".repeat(MAX_BYTES + 1)),
        Err(Error::Capacity)
    ));
}
#[test]
fn session_decode_validates_metadata_but_keeps_unknown_extensions_in_original_json() {
    let json = br#"{"stickers":[{"id":"s","x":0,"y":0,"w":20,"h":20,"captureMeta":{"dynamicRange":"hdr","bitDepth":16,"futureExtension":true}}],"links":[]}"#;
    let document = crate::session_document::decode(json).unwrap();
    assert_eq!(document.original_json(), json);
    assert_eq!(
        document.windows[0]
            .capture_metadata()
            .unwrap()
            .unwrap()
            .dynamic_range,
        Some(Range::Hdr)
    );
    let invalid = br#"{"stickers":[{"id":"s","x":0,"y":0,"w":20,"h":20,"captureMeta":{"dynamicRange":"hdr","bitDepth":8}}],"links":[]}"#;
    assert!(matches!(
        crate::session_document::decode(invalid),
        Err(crate::session_document::Error::Window)
    ));
    let missing: crate::session_window::Window<'_> =
        serde_json::from_str(r#"{"id":"s","x":0,"y":0,"w":20,"h":20}"#).unwrap();
    assert!(missing.capture_metadata().unwrap().is_none());
}
