//! 完整会话装配、候选和状态接管一起保护设置；不写用户会话文件。
use super::*;
use crate::{
    desktop::{session_binding::Bound, session_candidate},
    unit_node_settings::Settings,
};

#[test]
fn sticker_settings_roundtrip_through_save_candidate_and_real_state_restore() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let expected = Settings {
        hide_collapsed_endpoints: true,
        notify_downstream: false,
    };
    let mut document = doc("one");
    document.node_settings = Some(expected);
    let mut snapshot = data(vec![document]);
    snapshot.original = Source::from(
        serde_json::to_vec(&serde_json::json!({"stickers":[{
        "id":"one","x":10,"y":20,"w":40,"h":30,
        "executionConfig":{"propagation":{"notifyDownstream":true,"future":7},"future":"preserve"}
    }],"links":[]}))
        .unwrap(),
    );
    let saved = snapshot.encode(&cancel).unwrap();
    let value: Value = serde_json::from_slice(&saved.bytes).unwrap();
    let fields = &value["stickers"][0];
    assert_eq!(fields["hideCollapsedEndpoints"], true);
    assert_eq!(
        fields["executionConfig"]["propagation"]["notifyDownstream"],
        false
    );
    assert_eq!(fields["executionConfig"]["propagation"]["future"], 7);
    assert_eq!(fields["executionConfig"]["future"], "preserve");
    let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
    assert_eq!(candidate.placements[0].node_settings, expected);
    let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
    let states = Bound::prepare_for(vectors, None, &cancel)
        .ok()
        .unwrap()
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    assert_eq!(states.states[0].node_settings.get(), expected);
    assert!(states.states[0].accept_upstream.get());
    assert!(states.states[0].pending_start.borrow().is_none());
}
#[test]
fn changing_sticker_notify_back_to_enabled_is_saved_without_losing_trigger_metadata() {
    let cancel = AtomicBool::new(false);
    let mut snapshot = data(vec![doc("one")]);
    snapshot.original = Source::from(serde_json::to_vec(&serde_json::json!({"stickers":[{
        "id":"one","x":10,"y":20,"w":40,"h":30,"hideCollapsedEndpoints":true,
        "executionConfig":{"triggerMode":{"paramDriven":false},"propagation":{"notifyDownstream":false}}
    }],"links":[]})).unwrap());
    let saved = snapshot.assemble(&cancel, plain).unwrap();
    let saved: Value = serde_json::from_slice(&saved.bytes).unwrap();
    assert_eq!(saved["stickers"][0]["hideCollapsedEndpoints"], false);
    assert_eq!(
        saved["stickers"][0]["executionConfig"]["propagation"]["notifyDownstream"],
        true
    );
    assert_eq!(
        saved["stickers"][0]["executionConfig"]["triggerMode"]["paramDriven"],
        false
    );
}

#[test]
fn owners_without_node_settings_leave_existing_metadata_intact() {
    let cancel = AtomicBool::new(false);
    let mut document = doc("one");
    document.node_settings = None;
    let mut snapshot = data(vec![document]);
    snapshot.original = Source::from(
        serde_json::to_vec(&serde_json::json!({"stickers":[{
        "id":"one","x":10,"y":20,"w":40,"h":30,"hideCollapsedEndpoints":true,
        "executionConfig":{"propagation":{"notifyDownstream":false},"custom":"keep"}
    }],"links":[]}))
        .unwrap(),
    );
    let saved = snapshot.assemble(&cancel, plain).unwrap();
    let saved: Value = serde_json::from_slice(&saved.bytes).unwrap();
    assert_eq!(saved["stickers"][0]["hideCollapsedEndpoints"], true);
    assert_eq!(
        saved["stickers"][0]["executionConfig"]["propagation"]["notifyDownstream"],
        false
    );
    assert_eq!(saved["stickers"][0]["executionConfig"]["custom"], "keep");
}

#[test]
fn customized_node_settings_promote_an_otherwise_empty_transient_host() {
    for settings in [
        Settings::default(),
        Settings {
            hide_collapsed_endpoints: true,
            ..Default::default()
        },
        Settings {
            notify_downstream: false,
            ..Default::default()
        },
    ] {
        let mut document = doc("host");
        document.node_settings = Some(settings);
        let mut snapshot = data(vec![document]);
        snapshot.transient_host = Some(id("host"));
        let saved = snapshot.assemble(&AtomicBool::new(false), plain).unwrap();
        let read = crate::session_document::decode(&saved.bytes).unwrap();
        assert_eq!(
            read.windows.len(),
            usize::from(settings != Settings::default())
        );
    }
}
