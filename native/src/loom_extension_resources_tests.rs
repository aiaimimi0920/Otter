//! Check only locally prepared data boundaries, without file reads or system-clipboard access.
use super::*;
use crate::unit_attachment::PAYLOAD_BYTES;
use serde_json::json;
fn access() -> Access {
    Access {
        plugin: "neuro.official/ocr".into(),
        image: true,
        attachments: true,
        ocr_text: false,
    }
}
fn attachment() -> Value {
    json!({"attachmentId":"neuro.official/ocr.result", "typeId":"neuro.official/ocr.result.v1",
        "schemaVersion":"1", "revision":1, "pluginId":"neuro.official/ocr", "pluginVersion":"1.0.0", "payload":{"fullText":"测试"}})
}
#[test]
fn images_have_exact_base64_and_only_bounded_image_mime_types() {
    let value = serde_json::to_value(Image::new("IMAGE/PNG", &[1, 2, 3]).unwrap()).unwrap();
    assert_eq!(
        value,
        json!({"kind":"image", "mime":"image/png", "dataBase64":"AQID"})
    );
    for mime in [
        "text/plain",
        "image/",
        "image/png; charset=utf8",
        "image/png\n",
        " image/png",
    ] {
        assert!(matches!(Image::new(mime, &[1]), Err(Error::Wire)));
    }
    assert!(matches!(Image::new("image/png", &[]), Err(Error::Limit)));
    assert!(matches!(
        Image::new("image/png", &vec![0; 16 * 1024 * 1024 + 1]),
        Err(Error::Limit)
    ));
}
#[test]
fn opaque_attachments_remain_owner_scoped_and_host_preferences_are_not_uploaded() {
    let mut value = attachment();
    value["payloadDigest"] = json!("a".repeat(64));
    value["overlayVisible"] = json!(false);
    let prepared = Prepared::new(json!({}))
        .unwrap()
        .attachment(Attachment::parse(value).unwrap())
        .unwrap();
    let (_, images, attachments) = prepared.wire(&access()).unwrap();
    assert!(images.is_none());
    let value = &attachments.unwrap()[0];
    assert_eq!(value["payload"]["fullText"], "测试");
    assert!(value.get("payloadDigest").is_none() && value.get("overlayVisible").is_none());
    let mut foreign = access();
    foreign.plugin = "another/plugin".into();
    let prepared = Prepared::new(json!({}))
        .unwrap()
        .attachment(Attachment::parse(attachment()).unwrap())
        .unwrap();
    assert!(matches!(prepared.wire(&foreign), Err(Error::Denied)));
}
#[test]
fn undeclared_image_or_attachment_reads_cannot_be_added_to_a_prepared_request() {
    let mut denied = access();
    denied.image = false;
    assert!(matches!(
        Prepared::new(json!({}))
            .unwrap()
            .image(Image::new("image/png", &[1]).unwrap())
            .wire(&denied),
        Err(Error::Denied)
    ));
    denied.attachments = false;
    let prepared = Prepared::new(json!({}))
        .unwrap()
        .attachment(Attachment::parse(attachment()).unwrap())
        .unwrap();
    assert!(matches!(prepared.wire(&denied), Err(Error::Denied)));
}
#[test]
fn attachment_parse_enforces_identity_revision_resource_and_json_budgets() {
    for case in 0..9 {
        let mut value = attachment();
        match case {
            0 => value["attachmentId"] = json!("foreign.result"),
            1 => value["revision"] = json!(catalog::MAX_INTEGER + 1),
            2 => value["rendererId"] = Value::Null,
            3 => value["payload"] = json!("x".repeat(PAYLOAD_BYTES)),
            4 => {
                value["resourceRefs"] = json!([{"resourceId":"C:/private.png", "kind":"file", "digest":"a".repeat(64), "byteLength":1, "leaseId":"lease"}])
            }
            5 => value["schemaVersion"] = json!(""),
            6 => {
                value["resourceRefs"] = json!([{"resourceId":format!("sha256:{}", "a".repeat(64)), "kind":"file", "digest":"a".repeat(64), "byteLength":64 * 1024 * 1024 + 1, "leaseId":"lease"}])
            }
            7 => {
                value.as_object_mut().unwrap().remove("payload");
            }
            _ => {
                let mut deep = json!(0);
                for _ in 0..33 {
                    deep = json!([deep]);
                }
                value["payload"] = deep;
            }
        }
        assert!(Attachment::parse(value).is_err(), "case {case}");
    }
    let mut value = attachment();
    value["payload"] = Value::Null;
    assert_eq!(
        serde_json::to_value(Attachment::parse(value).unwrap()).unwrap()["payload"],
        Value::Null
    );
}
#[test]
fn invocation_attachment_count_and_aggregate_bytes_match_the_real_bridge_limits() {
    let mut prepared = Prepared::new(json!({})).unwrap();
    for _ in 0..32 {
        prepared = prepared
            .attachment(Attachment::parse(attachment()).unwrap())
            .unwrap();
    }
    assert!(matches!(
        prepared.attachment(Attachment::parse(attachment()).unwrap()),
        Err(Error::Limit)
    ));
    let mut large = attachment();
    large["payload"] = json!("x".repeat(100_000));
    let mut prepared = Prepared::new(json!({})).unwrap();
    for _ in 0..22 {
        prepared = prepared
            .attachment(Attachment::parse(large.clone()).unwrap())
            .unwrap();
    }
    assert!(matches!(prepared.wire(&access()), Err(Error::Limit)));
}
#[test]
fn attachment_resources_are_content_addressed_and_explicit_null_is_not_absence() {
    let mut value = attachment();
    value.as_object_mut().unwrap().remove("payload");
    value["resourceRefs"] = json!([{"resourceId":format!("sha256:{}", "a".repeat(64)), "kind":"shared_image", "digest":"a".repeat(64), "byteLength":64 * 1024 * 1024, "leaseId":"lease"}]);
    assert!(Attachment::parse(value.clone()).is_ok());
    value["resourceRefs"] = Value::Null;
    assert!(Attachment::parse(value).is_err());
}
