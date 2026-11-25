//! 来自实际 extension snapshot schema 的信任、归属、预算与条件回归；不使用真实扩展数据。
use super::*;
use crate::extension_when::UnitKind;
use serde_json::json;
pub fn fixture() -> Value {
    let mut lists = serde_json::Map::new();
    for name in LISTS {
        lists.insert(name.into(), json!([]));
    }
    lists.insert("commands".into(), json!([{
        "id":"ocr-overlay", "pluginId":"neuro.official/ocr", "scopeId":"scope:ocr",
        "commandId":"neuro.official/ocr.toggle-overlay", "when":"unit.kind == 'sticker' && unit.hasImage",
        "payload":{"schema":"loom.command.v1", "payload":{"permissions":["hook.unit.attachments.read", "hook.unit.attachments.write"]}}
    }]));
    json!({"protocol":PROTOCOL, "apiVersion":API, "generation":7,
        "plugins":[{"id":"neuro.official/ocr", "version":"1.0.0", "packageDigest":"a".repeat(64), "trustStatus":"trusted",
            "permissionGrantDigest":"b".repeat(64), "effectivePermissions":["hook.unit.attachments.read", "hook.unit.attachments.write"], "scopeId":"scope:ocr"}],
        "contributions":lists})
}
fn parse(value: Value) -> Result<Snapshot, Error> {
    Snapshot::parse(&serde_json::to_vec(&value).unwrap())
}
fn context() -> Context<'static> {
    Context {
        kind: Some(UnitKind::Sticker),
        has_image: true,
        attachment_types: &[],
    }
}
#[test]
fn trusted_matching_scope_and_when_project_real_command_and_permissions() {
    let snapshot = parse(fixture()).unwrap();
    let command = snapshot
        .command("neuro.official/ocr.toggle-overlay", &context())
        .unwrap();
    assert_eq!(snapshot.generation(), 7);
    assert_eq!(command.plugin(), "neuro.official/ocr");
    assert_eq!(command.scope(), "scope:ocr");
    assert!(command.permits("hook.unit.attachments.read"));
    assert!(!command.permits("hook.unit.image.read"));
    assert!(snapshot
        .command(
            command.id(),
            &Context {
                has_image: false,
                ..context()
            }
        )
        .is_none());
    for trust in ["untrusted", "revoked", "unsigned_developer"] {
        let mut data = fixture();
        data["plugins"][0]["trustStatus"] = json!(trust);
        assert!(parse(data)
            .unwrap()
            .command(command.id(), &context())
            .is_none());
    }
}
#[test]
fn declared_permissions_never_exceed_the_same_owners_grant() {
    for permissions in [
        json!(["hook.unit.attachments.read", "not.granted"]),
        json!([42]),
        json!(null),
        json!([]),
    ] {
        let mut data = fixture();
        data["contributions"]["commands"][0]["payload"]["payload"]["permissions"] = permissions;
        assert!(!parse(data)
            .unwrap()
            .command("neuro.official/ocr.toggle-overlay", &context())
            .unwrap()
            .permits("hook.unit.attachments.read"));
    }
    let mut data = fixture();
    data["plugins"][0]["effectivePermissions"] = Value::Null;
    assert!(!parse(data)
        .unwrap()
        .command("neuro.official/ocr.toggle-overlay", &context())
        .unwrap()
        .permits("hook.unit.attachments.read"));
}
#[test]
fn duplicate_ids_scope_mismatch_unknown_fields_and_invalid_digests_fail_at_boundary() {
    for case in 0..7 {
        let mut data = fixture();
        match case {
            0 => {
                let plugin = data["plugins"][0].clone();
                data["plugins"].as_array_mut().unwrap().push(plugin);
            }
            1 => data["contributions"]["commands"][0]["scopeId"] = json!("wrong"),
            2 => data["plugins"][0]["packageDigest"] = json!("A".repeat(64)),
            3 => data["plugins"][0]["effectivePermissions"] = json!(["same", "same"]),
            4 => data["futureRoot"] = json!(true),
            5 => data["contributions"]["future"] = json!([]),
            _ => {
                let mut other = data["contributions"]["commands"][0].clone();
                other["id"] = json!("OCR-OVERLAY");
                data["contributions"]["menus"] = json!([other]);
            }
        }
        assert!(parse(data).is_err(), "case {case}");
    }
}
#[test]
fn optional_null_is_not_absent_and_unknown_when_does_not_invalidate_other_commands() {
    for field in ["title", "commandId", "when", "placement", "order"] {
        let mut data = fixture();
        data["contributions"]["commands"][0][field] = Value::Null;
        assert!(parse(data).is_err(), "{field}");
    }
    let mut data = fixture();
    data["contributions"]["commands"][0]["when"] = json!("!unit.futureFlag");
    assert!(parse(data)
        .unwrap()
        .command("neuro.official/ocr.toggle-overlay", &context())
        .is_none());
}
#[test]
fn snapshot_version_safe_integers_and_all_count_and_byte_budgets_are_enforced() {
    for case in 0..7 {
        let mut data = fixture();
        match case {
            0 => data["apiVersion"] = json!("2.0"),
            1 => data["generation"] = json!(MAX_INTEGER + 1),
            2 => data["generation"] = json!(1.5),
            3 => {
                data["contributions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("shortcuts");
            }
            4 => data["plugins"] = Value::Array(vec![data["plugins"][0].clone(); 129]),
            5 => {
                data["contributions"]["commands"] =
                    Value::Array(vec![data["contributions"]["commands"][0].clone(); 257])
            }
            _ => data["contributions"]["commands"][0]["payload"] = json!("x".repeat(MAX_BYTES)),
        }
        assert!(parse(data).is_err(), "case {case}");
    }
    let mut data = fixture();
    data["apiVersion"] = json!("1.1");
    assert!(parse(data).is_ok());
}

#[test]
fn streaming_fingerprint_matches_legacy_compact_encoding() {
    let mut value = fixture();
    value["contributions"]["commands"][0]["payload"]["extra"] =
        json!({"z":[3.5, "目录\n\t😀"], "a":"quoted \" and slash \\"});
    let compact = serde_json::to_vec(&value).unwrap();
    let pretty = serde_json::to_vec_pretty(&value).unwrap();
    let expected = ring::digest::digest(&ring::digest::SHA256, &compact);
    let parsed = Snapshot::parse(&pretty).unwrap();
    assert_eq!(parsed.1.as_slice(), expected.as_ref());
    assert!(parsed.same_catalog(&Snapshot::parse(&compact).unwrap()));
}

#[test]
fn compact_budget_is_inclusive_and_original_typed_duplicates_remain_errors() {
    let mut value = fixture();
    value["contributions"]["commands"][0]["payload"] = json!("");
    let remaining = MAX_BYTES - serde_json::to_vec(&value).unwrap().len();
    value["contributions"]["commands"][0]["payload"] = json!("x".repeat(remaining));
    let pretty = serde_json::to_vec_pretty(&value).unwrap();
    assert!(pretty.len() > MAX_BYTES);
    assert!(Snapshot::parse(&pretty).is_ok());
    value["contributions"]["commands"][0]["payload"] = json!("x".repeat(remaining + 1));
    assert!(matches!(parse(value), Err(Error::Budget)));

    let original = fixture().to_string();
    for (key, duplicate) in [
        ("\"generation\":7", "\"generation\":7,\"generation\":7"),
        (
            "\"trustStatus\":\"trusted\"",
            "\"trustStatus\":\"trusted\",\"trustStatus\":\"trusted\"",
        ),
        (
            "\"id\":\"ocr-overlay\"",
            "\"id\":\"ocr-overlay\",\"id\":\"ocr-overlay\"",
        ),
    ] {
        assert!(original.contains(key));
        let duplicate = original.replacen(key, duplicate, 1);
        assert!(
            matches!(Snapshot::parse(duplicate.as_bytes()), Err(Error::Encoding)),
            "{key}"
        );
    }
}

#[test]
fn total_contribution_budget_is_independent_of_each_list_budget() {
    let mut data = fixture();
    let template = data["contributions"]["commands"][0].clone();
    for (index, name) in LISTS.iter().take(8).enumerate() {
        data["contributions"][name] = Value::Array(
            (0..256)
                .map(|item| {
                    let mut contribution = template.clone();
                    contribution["id"] = json!(format!("{index}:{item}"));
                    contribution.as_object_mut().unwrap().remove("commandId");
                    contribution
                })
                .collect(),
        );
    }
    assert!(parse(data.clone()).is_ok());
    let mut extra = template;
    extra["id"] = json!("extra");
    data["contributions"][LISTS[8]] = json!([extra]);
    assert!(matches!(parse(data), Err(Error::Budget)));
}
