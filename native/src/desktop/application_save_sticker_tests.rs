//! 保存真实图片输入并经过后台候选/State恢复；clear和禁用不能从旧原文复活。
use super::tests::{data, doc};
use super::*;
use crate::desktop::{session_binding::Bound, session_candidate, sticker_input, Apartment};
#[test]
fn save_reopen_and_clear_preserve_sticker_configuration_and_unknown_fields() {
    let _apartment = Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let asset = sticker_input::tests::asset();
    let raw=serde_json::value::RawValue::from_string(serde_json::json!({"image_path":asset.uri,"image_filename":asset.filename,"image":crate::graph_params::DISABLED,"future":{"keep":7}}).to_string()).unwrap();
    for clear in [false, true] {
        let control = sticker_input::Control::default();
        control.restore(sticker_input::Config::restore(Some(&raw), &cancel).unwrap());
        if clear {
            control.set_asset(None).unwrap();
        }
        let mut document = doc("sticker");
        document.sticker = Some((control.snapshot(), false));
        let mut snapshot = data(vec![document]);
        snapshot.transient_host = Some(crate::workspace_id::UnitId::parse("sticker").unwrap());
        snapshot.original=crate::session_source::Source::from(serde_json::to_vec(&serde_json::json!({"futureRoot":9,"stickers":[{"id":"sticker","x":0,"y":0,"w":40,"h":30,"params":serde_json::from_str::<Value>(raw.get()).unwrap(),"stickerEditPropagation":{"future":11}}],"links":[]})).unwrap());
        let saved = snapshot
            .assemble(&cancel, |_, fields| Ok(WindowCandidate::new(fields)))
            .unwrap();
        let json: Value = serde_json::from_slice(&saved.bytes).unwrap();
        assert_eq!(json["futureRoot"], 9);
        assert_eq!(json["stickers"][0]["stickerEditPropagation"]["future"], 11);
        assert_eq!(json["stickers"][0]["params"]["future"]["keep"], 7);
        assert_eq!(
            json["stickers"][0]["stickerEditPropagation"]["acceptUpstream"],
            false
        );
        assert_eq!(
            json["stickers"][0]["params"]["image_path"]
                .as_str()
                .unwrap_or("")
                .is_empty(),
            clear
        );
        let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
        let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
        let states = Bound::prepare_for(vectors, None, &cancel)
            .ok()
            .unwrap()
            .into_parts_for(None, &cancel)
            .ok()
            .unwrap()
            .prepare_states(&cancel)
            .unwrap();
        assert_eq!(states.states.len(), 1);
        let state = &states.states[0];
        assert!(!state.accept_upstream.get());
        let restored = state.sticker_input.snapshot();
        assert!(restored.disabled);
        assert_eq!(restored.asset.is_none(), clear);
        if let Some(loaded) = restored.asset {
            assert_eq!(loaded.image.pixels, asset.image.pixels);
        }
    }
}
#[test]
fn sticker_only_transient_host_is_kept_and_empty_default_host_is_omitted() {
    for configured in [false, true] {
        let mut document = doc("unit");
        let control = sticker_input::Control::default();
        if configured {
            control.toggle().unwrap();
        }
        document.sticker = Some((control.snapshot(), true));
        let mut snapshot = data(vec![document]);
        snapshot.transient_host = Some(crate::workspace_id::UnitId::parse("unit").unwrap());
        let saved = snapshot
            .assemble(&AtomicBool::new(false), |_, fields| {
                Ok(WindowCandidate::new(fields))
            })
            .unwrap();
        assert_eq!(
            crate::session_document::decode(&saved.bytes)
                .unwrap()
                .windows
                .len(),
            usize::from(configured)
        );
    }
}

#[test]
fn full_legacy_unknown_params_save_without_owner_key_growth() {
    let values: serde_json::Map<String, Value> = (0..128)
        .map(|i| (format!("future{i}"), Value::Null))
        .collect();
    let raw =
        serde_json::value::RawValue::from_string(serde_json::to_string(&values).unwrap()).unwrap();
    let mut document = doc("full");
    document.sticker = Some((
        sticker_input::Config::restore(Some(&raw), &AtomicBool::new(false)).unwrap(),
        true,
    ));
    let saved = data(vec![document])
        .assemble(&AtomicBool::new(false), |_, fields| {
            Ok(WindowCandidate::new(fields))
        })
        .unwrap();
    let json: Value = serde_json::from_slice(&saved.bytes).unwrap();
    assert_eq!(json["stickers"][0]["params"], Value::Object(values));
}
