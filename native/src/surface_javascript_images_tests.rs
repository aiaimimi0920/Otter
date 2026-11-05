//! 安装协议的声明/实际内容/中断边界；预期PNG由测试png库构造，不依赖Canvas编码。
use super::*;
pub(crate) fn resource() -> ImageResource {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 128,
            ])
            .unwrap();
    }
    ImageResource {
        id: "input:1".into(),
        width: 2,
        height: 2,
        png: png.into(),
    }
}
#[test]
fn headers_reject_duplicate_identity_counts_dimensions_and_budget_before_read() {
    let image = resource();
    let headers = headers(&[image]).unwrap();
    assert!(validate(&headers).is_ok());
    let original = serde_json::to_value(&headers).unwrap();
    for (key, value) in [
        ("id", serde_json::json!("../escape")),
        ("size", serde_json::json!([0, 1])),
        ("size", serde_json::json!([4097, 1])),
        ("size", serde_json::json!([4096, 4096])),
        ("bytes", serde_json::json!(0)),
        ("bytes", serde_json::json!(MAX_PNG + 1)),
        ("digest", serde_json::json!("A".repeat(64))),
    ] {
        let mut changed = original.clone();
        changed[0][key] = value;
        assert!(
            validate(&serde_json::from_value::<Vec<Header>>(changed).unwrap()).is_err(),
            "{key}"
        );
    }
    assert!(validate(&[]).is_err());
    let mut duplicate = original.as_array().unwrap().clone();
    duplicate.push(duplicate[0].clone());
    assert!(validate(
        &serde_json::from_value::<Vec<Header>>(serde_json::json!(duplicate)).unwrap()
    )
    .is_err());
    let mut many = Vec::new();
    for i in 0..9 {
        let mut h = original[0].clone();
        h["id"] = serde_json::json!(format!("image:{i}"));
        many.push(h);
    }
    assert!(
        validate(&serde_json::from_value::<Vec<Header>>(serde_json::json!(many)).unwrap()).is_err()
    );
    let mut big = original[0].clone();
    big["size"] = serde_json::json!([2048, 2048]);
    let mut other = big.clone();
    other["id"] = serde_json::json!("other");
    assert!(validate(
        &serde_json::from_value::<Vec<Header>>(serde_json::json!([big, other])).unwrap()
    )
    .is_err());
}
#[test]
fn raw_payload_rejects_truncation_wrong_digest_and_preserves_next_frame_boundary() {
    let images = vec![resource()];
    let headers = headers(&images).unwrap();
    let mut bytes = Vec::new();
    write(&mut bytes, &images).unwrap();
    let original = bytes.clone();
    super::super::write(&mut bytes, &Installed { sequence: 2 }).unwrap();
    let mut reader = std::io::Cursor::new(bytes);
    assert_eq!(read(&mut reader, &headers[0]).unwrap(), images[0].png);
    assert_eq!(
        super::super::read::<Installed>(&mut reader)
            .unwrap()
            .sequence,
        2
    );
    assert!(read(&mut &original[..original.len() - 1], &headers[0]).is_err());
    let mut bad = original;
    bad[20] ^= 1;
    assert!(read(&mut &bad[..], &headers[0]).is_err());
}
