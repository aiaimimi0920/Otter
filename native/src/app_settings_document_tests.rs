//! 合成配置的兼容、容量与失败不变性；不访问用户配置目录。
use super::*;
use serde_json::json;

#[test]
fn recycle_edit_preserves_other_fields_and_invalid_edit_is_atomic() {
    use crate::session_library::recycle::policy::Policy;
    let original = json!({"schemaVersion":1,"extension":[true],"cache":{
        "tempCacheMaxBytes":268435456,"future":{"keep":true}
    }});
    let mut doc = Document::decode(&serde_json::to_vec(&original).unwrap()).unwrap();
    let policy = Policy {
        recycle_bin_max_entries: 0,
        recycle_bin_retention_days: 3650,
    };
    doc.set_recycle_policy(policy).unwrap();
    assert_eq!(doc.recycle_policy().unwrap(), policy);
    let before = doc.encode().unwrap();
    let encoded: Value = serde_json::from_slice(&before).unwrap();
    for key in ["tempCacheMaxBytes", "future"] {
        assert_eq!(encoded["cache"][key], original["cache"][key]);
    }
    assert_eq!(encoded["extension"], original["extension"]);
    assert_eq!(encoded["schemaVersion"], 1);
    for invalid in [
        Policy {
            recycle_bin_max_entries: 501,
            ..policy
        },
        Policy {
            recycle_bin_retention_days: 3651,
            ..policy
        },
    ] {
        assert!(doc.set_recycle_policy(invalid).is_err());
        assert_eq!(doc.encode().unwrap(), before);
    }
}

#[test]
fn naming_edit_preserves_other_owners_and_unknown_fields() {
    let original = json!({
        "schemaVersion": 2,
        "cache": {"tempCacheMaxBytes": 268435456},
        "translationProviderMode": "gateway",
        "extension": [true, null, {"名称": "保留"}],
        "fileNaming": {"stickerSavePattern": "旧_{shortId}", "futureField": {"x": 5}}
    });
    let mut doc = Document::decode(&serde_json::to_vec(&original).unwrap()).unwrap();
    let mut naming = doc.naming().clone();
    naming.sticker_save_pattern = "新_{date}_{width}x{height}".into();
    doc.set_naming(naming.clone()).unwrap();
    let bytes = doc.encode().unwrap();
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    let encoded: Value = serde_json::from_slice(&bytes).unwrap();
    for key in [
        "schemaVersion",
        "cache",
        "translationProviderMode",
        "extension",
    ] {
        assert_eq!(encoded[key], original[key]);
    }
    assert_eq!(
        encoded["fileNaming"]["futureField"],
        original["fileNaming"]["futureField"]
    );
    assert_eq!(Document::decode(&bytes).unwrap().naming(), &naming);
}

#[test]
fn missing_fields_default_without_implicit_upgrade_or_rewrite() {
    for input in [r#"{}"#, r#"{"schemaVersion":1}"#, r#"{"schemaVersion":2}"#] {
        let doc = Document::decode(input.as_bytes()).unwrap();
        assert_eq!(doc.naming(), &Settings::default());
        assert_eq!(
            serde_json::from_slice::<Value>(&doc.encode().unwrap()).unwrap(),
            serde_json::from_str::<Value>(input).unwrap()
        );
    }
}

#[test]
fn unsupported_or_malformed_configuration_is_not_defaulted() {
    for input in [
        "null",
        "[]",
        "{",
        r#"{"schemaVersion":3}"#,
        r#"{"schemaVersion":0}"#,
        r#"{"schemaVersion":null}"#,
        r#"{"schemaVersion":"2"}"#,
        r#"{"schemaVersion":2.0}"#,
        r#"{"fileNaming":null}"#,
        r#"{"fileNaming":[]}"#,
        r#"{"fileNaming":{"titleMaxLength":0}}"#,
        r#"{"fileNaming":{"stickerSavePattern":"{bad}"}}"#,
        r#"{"fileNaming":{"collisionPolicy":"overwrite"}}"#,
    ] {
        assert!(Document::decode(input.as_bytes()).is_err(), "{input}");
    }
    assert!(Document::decode(&[0xff]).is_err());
    assert!(Document::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
}

#[test]
fn failed_edit_keeps_original_document_and_runtime_settings() {
    let mut doc = Document::decode(br#"{"extra":17}"#).unwrap();
    let before = doc.encode().unwrap();
    let invalid = Settings {
        title_max_length: 241,
        ..Default::default()
    };
    assert!(doc.set_naming(invalid).is_err());
    assert_eq!(doc.encode().unwrap(), before);
    assert_eq!(doc.naming(), &Settings::default());
}

#[test]
fn output_limit_is_checked_after_edit_without_truncation() {
    let input = format!(r#"{{"extra":"{}"}}"#, "x".repeat(MAX_BYTES - 12));
    assert_eq!(input.len(), MAX_BYTES);
    let mut doc = Document::decode(input.as_bytes()).unwrap();
    assert_eq!(doc.encode().unwrap().len(), MAX_BYTES);
    doc.set_naming(Settings::default()).unwrap();
    assert!(doc.encode().is_err());
}
