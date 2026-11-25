//! 插件声明、修饰键和冲突预算；不触碰系统全局注册。
use super::*;
use serde_json::json;
fn fixture() -> Value {
    let mut v = crate::loom_extension_snapshot::tests::fixture();
    v["contributions"]["shortcuts"] = json!([
        {"id":"ocr-run", "pluginId":"neuro.official/ocr", "scopeId":"scope:ocr", "commandId":"neuro.official/ocr.toggle-overlay",
         "when":"unit.hasImage", "payload":{"keys":"Ctrl+4 / Alt+4","global":true}},
        {"id":"z-duplicate", "pluginId":"neuro.official/ocr", "scopeId":"scope:ocr", "commandId":"neuro.official/ocr.toggle-overlay",
         "payload":{"keys":"Ctrl+4 / Ctrl+1 / Ctrl+C / Ctrl+Alt+Space / F2 / F6 / Ctrl+E"}}
    ]);
    v
}
fn parse(v: Value) -> Snapshot {
    Snapshot::parse(&serde_json::to_vec(&v).unwrap()).unwrap()
}
#[test]
fn ordered_alternatives_aliases_reserved_and_duplicate_chords_match_the_native_contract() {
    let s = parse(fixture());
    let b = s.shortcuts().unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!(
        b[0].chord,
        Chord {
            key: b'4' as u32,
            modifiers: 2
        }
    );
    assert_eq!(
        b[1].chord,
        Chord {
            key: b'4' as u32,
            modifiers: 1
        }
    );
    assert!(b.iter().all(|b| b.global && b.id == "ocr-run"));
    assert_eq!(
        chord("Control+WIN+f12"),
        Some(Chord {
            key: 0x7b,
            modifiers: 10
        })
    );
    assert!(chord("Ctrl+滚轮").is_none());
}
#[test]
fn malformed_alternatives_do_not_register_or_steal_valid_chords() {
    for bad in ["Ctrl+4+5", "Ctrl+4+4", "Ctrl++5", "+Ctrl+5", "Ctrl+5+"] {
        let mut v = fixture();
        v["contributions"]["shortcuts"][0]["payload"]["keys"] = format!("{bad} / Alt+4").into();
        v["contributions"]["shortcuts"][1]["payload"]["keys"] = "Ctrl+5".into();
        let definitions = parse(v).shortcuts().unwrap();
        let keys: Vec<_> = definitions
            .iter()
            .map(|d| (d.id.as_str(), d.chord, d.global))
            .collect();
        assert_eq!(
            keys,
            [
                (
                    "ocr-run",
                    Chord {
                        key: b'4' as u32,
                        modifiers: 1
                    },
                    true
                ),
                (
                    "z-duplicate",
                    Chord {
                        key: b'5' as u32,
                        modifiers: 2
                    },
                    false
                ),
            ],
            "{bad}"
        );
    }
}
#[test]
fn owner_scope_trust_command_and_both_when_predicates_are_rechecked() {
    let ctx = Context {
        kind: Some(crate::extension_when::UnitKind::Sticker),
        has_image: true,
        attachment_types: &[],
    };
    let s = parse(fixture());
    assert!(s.shortcut_available("ocr-run", &ctx));
    assert!(!s.shortcut_available(
        "ocr-run",
        &Context {
            has_image: false,
            ..ctx
        }
    ));
    let mut v = fixture();
    v["contributions"]["shortcuts"][0]["commandId"] = "missing".into();
    assert!(!parse(v)
        .shortcuts()
        .unwrap()
        .iter()
        .any(|b| b.id == "ocr-run"));
    for trust in ["revoked", "untrusted", "unsigned_developer"] {
        let mut v = fixture();
        v["plugins"][0]["trustStatus"] = trust.into();
        assert!(parse(v).shortcuts().unwrap().is_empty());
    }
}
#[test]
fn oversized_alternative_payload_is_rejected_instead_of_allocating_an_unbounded_key_list() {
    let mut v = fixture();
    v["contributions"]["shortcuts"][0]["payload"]["keys"] = "Ctrl+4/".repeat(1000).into();
    assert!(matches!(parse(v).shortcuts(), Err(Error::Budget)));
}
