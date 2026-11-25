//! The large-image exception belongs only to image_path; unknown parameters, Art, and whole-session budgets are not expanded.
use super::*;
use serde_json::json;
fn read_value(value: Value) -> Result<Map<String, Value>, &'static str> {
    let raw = serde_json::value::RawValue::from_string(value.to_string()).unwrap();
    read(Some(&raw))
}
#[test]
fn large_inline_image_is_allowed_without_relaxing_other_parameters() {
    let uri = format!("data:image/png;base64,{}", "AAAA".repeat(300_000));
    let values =
        read_value(json!({"image_path":uri,"image_filename":"中文.png","future":{"keep":17}}))
            .unwrap();
    assert_eq!(values["future"]["keep"], 17);
    assert!(crate::loom_parameter_state::validate(
        values.iter().map(|(k, v)| (k.as_str(), Some(v)))
    )
    .is_err());
    assert!(read_value(json!({"image_path":"","future":"x".repeat(1024 * 1024)})).is_err());
}
#[test]
fn path_network_invalid_base64_and_invalid_metadata_are_rejected() {
    for value in [
        json!(null),
        json!([]),
        json!({"image_path":false}),
        json!({"image_path":"C:/private.png"}),
        json!({"image_path":"https://invalid/image.png"}),
        json!({"image_path":"data:image/gif;base64,AAAA"}),
        json!({"image_path":"data:image/png;base64,%%%%"}),
        json!({"image_path":"data:image/png;base64,AAAA===="}),
        json!({"image_filename":"bad\n.png"}),
    ] {
        assert!(read_value(value).is_err());
    }
    assert!(image_parts(&format!(
        "data:image/png;base64,{}",
        "A".repeat(MAX_FILE_BYTES.div_ceil(3) * 4)
    ))
    .is_err());
}
#[test]
fn aliases_clear_and_parameter_count_follow_the_dedicated_contract() {
    for (mime, expected) in [
        ("image/jpg", "image/jpeg"),
        ("image/pjpeg", "image/jpeg"),
        ("image/x-ms-bmp", "image/bmp"),
    ] {
        let uri = format!("data:{mime};base64,AAAA");
        assert_eq!(image_parts(&uri).unwrap().0, expected);
    }
    assert!(
        read_value(json!({"image_path":"","image_filename":"","image":"__DISABLED__"})).is_ok()
    );
    let mut values = Map::new();
    values.insert("image_path".into(), json!(""));
    for index in 0..128 {
        values.insert(format!("key{index}"), Value::Null);
    }
    assert!(validate(&values).is_err());
}

#[test]
fn checked_image_borrows_the_admitted_payload_only_after_parameter_validation() {
    let uri = "data:image/x-ms-bmp;base64,AAAA";
    let raw = serde_json::value::to_raw_value(&json!({"image_path":uri})).unwrap();
    let (values, ()) = read_with_image(Some(&raw), |values, image| {
        let (mime, encoded) = image.unwrap().parts();
        assert_eq!(mime, "image/bmp");
        let stored = values["image_path"].as_str().unwrap();
        let payload = stored.strip_prefix("data:image/x-ms-bmp;base64,").unwrap();
        assert_eq!(encoded, payload);
        assert_eq!(encoded.as_ptr(), payload.as_ptr());
    })
    .unwrap();
    assert_eq!(values["image_path"], uri);
    for invalid in [
        json!({"image_path":uri,"image_filename":"bad\nname"}),
        json!({"image_path":"data:image/png;base64,%%%%"}),
        json!({"image_path":uri,"unknown":"x".repeat(crate::loom_parameter_state::MAX_BYTES)}),
    ] {
        let raw = serde_json::value::to_raw_value(&invalid).unwrap();
        assert!(
            read_with_image::<()>(Some(&raw), |_, _| panic!("invalid params exposed image"))
                .is_err()
        );
    }
    let raw = serde_json::value::to_raw_value(&json!({"image_path":""})).unwrap();
    read_with_image(Some(&raw), |_, image| assert!(image.is_none())).unwrap();
}
