//! 真实 State 与合成能力目录的命名上下文；不读取进程、窗口标题或执行能力。
use super::*;
use std::rc::Rc;
fn image_state() -> State {
    State::new(
        Some(crate::desktop::image::Image {
            width: 2,
            height: 3,
            pixels: vec![255; 24].into(),
        }),
        false,
    )
    .unwrap()
}
fn selection(label: &str) -> crate::desktop::capability_selection::Selection {
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["label"] = serde_json::Value::from(label);
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    crate::desktop::capability_selection::Selection {
        execution: Default::default(),
        identity: crate::desktop::document_state::new_unit_id().unwrap(),
        catalog: Rc::new(crate::desktop::capability_job::Loaded {
            identity,
            handshake,
        }),
        parameters,
    }
}

#[test]
fn png_suggestion_uses_capability_label_title_limit_and_art_id() {
    let state = image_state();
    *state.selected_art.borrow_mut() = Some(selection(" 节点🙂名称 "));
    {
        let mut settings = state.naming.borrow_mut();
        settings.sticker_save_pattern = "{label}_{title}_{process}_{kind}_{width}x{height}".into();
        settings.title_max_length = 2;
    }
    assert_eq!(
        crate::desktop::export_naming::suggested(&state).unwrap(),
        "节点🙂名称_节点_test-art_art_2x3.png"
    );
}

#[test]
fn image_and_offline_art_have_distinct_fallbacks_without_os_queries() {
    let mut state = image_state();
    state.naming.borrow_mut().sticker_save_pattern = "{label}_{title}_{process}_{kind}".into();
    assert_eq!(
        crate::desktop::export_naming::suggested(&state).unwrap(),
        "image___image.png"
    );
    state.restored_art = true;
    state.restored_naming_art = RestoredArtId::from_id(Some("offline-id"));
    assert_eq!(
        crate::desktop::export_naming::suggested(&state).unwrap(),
        "art__offline-id_art.png"
    );
}

#[test]
fn live_selection_overrides_restored_id_and_oversized_fallback_is_not_copied() {
    let mut state = image_state();
    state.restored_art = true;
    state.restored_naming_art = RestoredArtId::from_id(Some(&"x".repeat(MAX_CONTEXT_BYTES + 1)));
    assert!(matches!(
        state.restored_naming_art,
        RestoredArtId::Oversized
    ));
    assert!(crate::desktop::export_naming::suggested(&state).is_none());
    *state.selected_art.borrow_mut() = Some(selection("current"));
    state.naming.borrow_mut().sticker_save_pattern = "{process}_{title}".into();
    assert_eq!(
        crate::desktop::export_naming::suggested(&state).unwrap(),
        "test-art_current.png"
    );
}
