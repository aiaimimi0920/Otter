//! 正式checkpoint独立oracle：解码持久PNG为straight RGBA，而非复用Art图像适配器。
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
pub fn ready(document: &Value, mode: &str) -> bool {
    document["stickers"]
        .as_array()
        .is_some_and(|units| units.iter().any(|u| u["outputs"]["mode"] == mode))
}
pub fn verify(document: &Value, source: &Value, mode: &str) -> String {
    assert_eq!(document["extension"]["productArtPreserved"], true);
    let units = document["stickers"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    let original = units.iter().find(|u| u["id"] == "product-source").unwrap();
    assert_eq!(decode(&original["src"]), decode(source));
    assert!(original
        .get("outputs")
        .is_none_or(|v| v.is_null() || v.as_object().is_some_and(|v| v.is_empty())));
    let art = units
        .iter()
        .find(|u| u["artId"] == super::package::ART)
        .unwrap();
    assert_eq!(art["outputs"]["answer"], 42);
    assert_eq!(art["outputs"]["mode"], mode);
    let (width, height, kind, rgba) = decode(&art["outputs"]["output_image"]);
    assert_eq!((width, height, kind), (2, 2, png::ColorType::Rgba));
    assert_eq!(
        rgba,
        [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 128]
    );
    let target = art["id"].as_str().unwrap();
    let links = document["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["fromUnitId"], "product-source");
    assert_eq!(links[0]["toUnitId"], target);
    target.into()
}
pub(super) fn decode(value: &Value) -> (u32, u32, png::ColorType, Vec<u8>) {
    let encoded = value
        .as_str()
        .unwrap()
        .strip_prefix("data:image/png;base64,")
        .unwrap();
    let bytes = STANDARD.decode(encoded).unwrap();
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut rgba = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut rgba).unwrap();
    rgba.truncate(info.buffer_size());
    (info.width, info.height, info.color_type, rgba)
}
