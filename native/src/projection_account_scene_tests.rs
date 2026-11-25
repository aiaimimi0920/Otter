//! 新账号关联的场景边界；合成签名不提供网络授权，测试真实原文 CAS 和候选校验。
use super::*;
use crate::{session_projection_scope::Scope, session_source::Source, workspace_id::UnitId};

pub(crate) fn fixture(scope: Scope, unit: &str) -> (Content, scene::Binding) {
    let image = image(17);
    let content = Content {
        envelope: envelope(&image),
        revision: 1,
        digest: image.digest().into(),
        linked: true,
        receiver_device_id: Some(RECEIVER.into()),
        receiver_unit_id: Some(unit.into()),
        image: Some(image),
        transport: "direct".into(),
        error: None,
    };
    let binding = scene::Binding::received(
        &content,
        &context(RECEIVER),
        "https://127.0.0.1:54443",
        &UnitId::parse(unit).unwrap(),
        scope,
    )
    .unwrap();
    (content, binding)
}
pub(crate) fn source_fixture(origin: &str) -> scene::Binding {
    let (content, _) = fixture(scope(), "receive-1");
    scene::Binding::sent(&content, &context(SOURCE), origin, scope()).unwrap()
}
fn scope() -> Scope {
    serde_json::from_value(json!("a".repeat(64))).unwrap()
}
#[test]
fn cross_account_receiver_binding_survives_save_and_refresh_without_source_privileges() {
    let (mut content, _) = fixture(scope(), "receive-1");
    let mut receiver = context(RECEIVER);
    receiver.account_id = "another-account".into();
    let binding = scene::Binding::received(
        &content,
        &receiver,
        "https://receiver-loom.example",
        &UnitId::parse("receive-1").unwrap(),
        scope(),
    )
    .unwrap();
    let stored = serde_json::to_string(&binding).unwrap();
    let recovered = scene::Binding::decode(&stored, "receive-1", scope()).unwrap();
    content.revision = 2;
    content.digest = image(211).digest().into();
    assert_eq!(
        recovered.follows(&content, &receiver).unwrap().revision(),
        2
    );
    assert!(scene::Binding::sent(&content, &receiver, binding.origin(), scope()).is_err());
    receiver.device_id = SOURCE.into();
    assert!(recovered.follows(&content, &receiver).is_err());
    assert!(scene::Binding::sent(&content, &receiver, binding.origin(), scope()).is_err());
}
fn source() -> Source {
    serde_json::to_vec(&json!({"documentRevision":7,"otterNativeProjectionScope":scope(),
        "future":{"keep":[1,2,3]},"stickers":[
        {"id":"receive-1","type":"sticker","x":0,"y":0,"w":2,"h":1,"params":{},"future":{"keep":true}},
        {"id":"other-1","type":"sticker","x":4,"y":5,"w":2,"h":1,"params":{}}],"links":[]}))
        .unwrap().into()
}
#[test]
fn scene_binding_requires_actual_receiver_unit_and_preserves_unknown_scene_fields() {
    let (mut content, binding) = fixture(scope(), "receive-1");
    let source = source();
    let next = binding
        .replace(&source, None, &AtomicBool::new(false))
        .unwrap();
    let before: Value = serde_json::from_slice(&source).unwrap();
    let after: Value = serde_json::from_slice(&next).unwrap();
    assert_eq!(after["documentRevision"], 7);
    assert_eq!(after["future"], before["future"]);
    assert_eq!(
        after["stickers"][0]["future"],
        before["stickers"][0]["future"]
    );
    assert_eq!(after["stickers"][1], before["stickers"][1]);
    let index = scene::Index::capture(&next).unwrap();
    assert!(index.includes(&binding) && index.is_for(&next.clone()));
    assert!(!index.is_for(&next.to_vec().into()));
    content.receiver_unit_id = Some("other-1".into());
    assert!(scene::Binding::received(
        &content,
        &context(RECEIVER),
        binding.origin(),
        &UnitId::parse("receive-1").unwrap(),
        scope()
    )
    .is_err());
}
#[test]
fn shared_document_keeps_the_exact_source_and_requires_its_scope_proof() {
    let (_, binding) = fixture(scope(), "receive-1");
    let bound = binding
        .replace(&source(), None, &AtomicBool::new(false))
        .unwrap();
    let parsed = bound.parse().unwrap();
    let independent: Source = bound.to_vec().into();
    assert_eq!(bound.as_slice(), independent.as_slice());
    let wrong = crate::session_projection_scope::Proof::capture(&independent).unwrap();
    assert!(scene::Index::capture_parsed(&parsed, Some(&wrong)).is_err());
    assert!(scene::Index::capture_parsed(&parsed, None).is_err());
    let proof = crate::session_projection_scope::Proof::capture_parsed(&parsed).unwrap();
    let index = scene::Index::capture_parsed(&parsed, Some(&proof)).unwrap();
    assert!(index.is_for(&bound) && index.includes(&binding));
    assert!(!index.is_for(&independent));
}

#[test]
fn scene_cas_rejects_stale_previous_regression_digest_change_and_cancel() {
    let (mut content, initial) = fixture(scope(), "receive-1");
    let cancel = AtomicBool::new(false);
    let original = source();
    let first = initial.replace(&original, None, &cancel).unwrap();
    assert!(initial.replace(&first, None, &cancel).is_err());
    assert!(initial.replace(&original, Some(&initial), &cancel).is_err());
    let second_image = image(211);
    content.revision = 2;
    content.digest = second_image.digest().into();
    let next = scene::Binding::received(
        &content,
        &context(RECEIVER),
        initial.origin(),
        &UnitId::parse("receive-1").unwrap(),
        scope(),
    )
    .unwrap();
    let second = next.replace(&first, Some(&initial), &cancel).unwrap();
    assert!(initial.replace(&second, Some(&next), &cancel).is_err());
    content.digest = "f".repeat(64);
    let conflict = scene::Binding::received(
        &content,
        &context(RECEIVER),
        initial.origin(),
        &UnitId::parse("receive-1").unwrap(),
        scope(),
    )
    .unwrap();
    assert!(conflict.replace(&second, Some(&next), &cancel).is_err());
    assert!(matches!(
        initial.replace(&original, None, &AtomicBool::new(true)),
        Err(Error::Stopped)
    ));
}
#[test]
fn scene_decode_rejects_wrong_scope_unit_unknown_fields_and_duplicate_invitation() {
    let (_, binding) = fixture(scope(), "receive-1");
    let text = serde_json::to_string(&binding).unwrap();
    assert!(scene::Binding::decode(&text, "other-1", scope()).is_err());
    let other_scope = serde_json::from_value(json!("b".repeat(64))).unwrap();
    assert!(scene::Binding::decode(&text, "receive-1", other_scope).is_err());
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["unknown"] = true.into();
    assert!(scene::Binding::decode(&value.to_string(), "receive-1", scope()).is_err());
    let bound = binding
        .replace(&source(), None, &AtomicBool::new(false))
        .unwrap();
    let mut value: Value = serde_json::from_slice(&bound).unwrap();
    let mut duplicate = serde_json::to_value(&binding).unwrap();
    duplicate["localUnit"] = "other-1".into();
    value["stickers"][1][scene::FIELD] = duplicate;
    assert!(scene::Index::capture(&serde_json::to_vec(&value).unwrap().into()).is_err());
}

#[test]
fn source_binding_replacement_requires_confirmed_stop_and_preserves_receiver_image_metadata() {
    let (content, _) = fixture(scope(), "receive-1");
    let binding = scene::Binding::sent(
        &content,
        &context(SOURCE),
        "https://127.0.0.1:54443",
        scope(),
    )
    .unwrap();
    assert!(binding.is_source());
    let mut scene: Value = serde_json::from_slice(&source()).unwrap();
    scene["stickers"][0]["id"] = binding.unit().into();
    let original: Source = serde_json::to_vec(&scene).unwrap().into();
    let cancel = AtomicBool::new(false);
    let first = binding.replace(&original, None, &cancel).unwrap();
    let mut different = content;
    different.envelope = envelope(&image(18));
    different.digest = different.envelope.digest().into();
    let next =
        scene::Binding::sent(&different, &context(SOURCE), binding.origin(), scope()).unwrap();
    assert!(next.replace(&first, Some(&binding), &cancel).is_err());
    let stopped = binding.stop();
    let retired = stopped.replace(&first, Some(&binding), &cancel).unwrap();
    let replaced = next.replace(&retired, Some(&stopped), &cancel).unwrap();
    assert!(scene::Index::capture(&replaced).unwrap().includes(&next));
    assert!(next
        .replace_image(&replaced, &next, "data:image/png;base64,AA==", &cancel)
        .is_err());
}
