use super::*;
use crate::app_settings_document::Document;
#[test]
fn cache_merge_is_validated_and_preserves_unknown_configuration() {
    let mut document = Document::decode(
        br#"{"schemaVersion":1,"cache":{"recycleBinMaxEntries":123,"extension":true}}"#,
    )
    .unwrap();
    let old = document.encode().unwrap();
    assert!(document
        .set_cache_policy(Policy {
            temp_cache_max_bytes: 1,
            temp_cache_retention_days: 7
        })
        .is_err());
    assert_eq!(document.encode().unwrap(), old);
    let policy = Policy {
        temp_cache_max_bytes: 0,
        temp_cache_retention_days: 0,
    };
    document.set_cache_policy(policy).unwrap();
    assert_eq!(document.cache_policy().unwrap(), policy);
    let value: serde_json::Value = serde_json::from_slice(&document.encode().unwrap()).unwrap();
    assert_eq!(value["cache"]["recycleBinMaxEntries"], 123);
    assert_eq!(value["cache"]["extension"], true);
    assert_eq!(value["schemaVersion"], 1);
}
#[test]
fn defaults_zero_and_partial_cache_settings_match_main_without_losing_other_fields() {
    assert_eq!(
        Document::default().cache_policy().unwrap(),
        Policy::default()
    );
    let mut document = Document::decode(br#"{"cache":{"tempCacheMaxBytes":0,"tempCacheRetentionDays":0,"recycleBinMaxEntries":123,"extension":true}}"#).unwrap();
    assert_eq!(
        document.cache_policy().unwrap(),
        Policy {
            temp_cache_max_bytes: 0,
            temp_cache_retention_days: 0
        }
    );
    document.set_naming(Default::default()).unwrap();
    let output: serde_json::Value = serde_json::from_slice(&document.encode().unwrap()).unwrap();
    assert_eq!(output["cache"]["recycleBinMaxEntries"], 123);
    assert_eq!(output["cache"]["extension"], true);
    let partial = Document::decode(br#"{"cache":{"tempCacheRetentionDays":30}}"#)
        .unwrap()
        .cache_policy()
        .unwrap();
    assert_eq!(partial.temp_cache_max_bytes, 256 * 1024 * 1024);
    assert_eq!(partial.temp_cache_retention_days, 30);
}
#[test]
fn invalid_cache_values_are_explicit_errors_not_silent_defaults() {
    for raw in [
        r#"{"cache":null}"#,
        r#"{"cache":[]}"#,
        r#"{"cache":{"tempCacheRetentionDays":3651}}"#,
        r#"{"cache":{"tempCacheRetentionDays":-1}}"#,
        r#"{"cache":{"tempCacheMaxBytes":1}}"#,
        r#"{"cache":{"tempCacheMaxBytes":17179869185}}"#,
        r#"{"cache":{"tempCacheMaxBytes":"0"}}"#,
    ] {
        assert!(
            Document::decode(raw.as_bytes())
                .unwrap()
                .cache_policy()
                .is_err(),
            "{raw}"
        );
    }
    for bytes in [32 * 1024 * 1024, 16 * 1024 * 1024 * 1024] {
        assert!(Policy {
            temp_cache_max_bytes: bytes,
            temp_cache_retention_days: 3650
        }
        .validate()
        .is_ok());
    }
}
