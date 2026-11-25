use super::*;
#[test]
fn restored_metadata_keeps_extensions_but_does_not_grant_file_lease_or_claim_original_availability()
{
    let raw = r#"{"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq","futureExtension":{"keep":true}}"#;
    let source = CaptureSource::from_session(raw).unwrap();
    assert!(source.original.is_none());
    assert!(source.original_lease.is_none());
    assert_eq!(
        source.metadata(),
        serde_json::from_str::<serde_json::Value>(raw).unwrap()
    );
    let text = describe(&source);
    assert_eq!(
        text,
        message(
            "capture-info-summary",
            &[
                ("original", source.original_label().into()),
                ("downgraded", tr("capture-info-unrecorded").into()),
                ("reason", reason(source.reason).into()),
            ]
        )
    );
    assert!(CaptureSource::from_session(r#"{"dynamicRange":"hdr","bitDepth":8}"#).is_err());
}
fn source() -> CaptureSource {
    CaptureSource {
        session_meta: None,
        original_lease: None,
        original: None,
        reason: Some(Reason::HdrFailed),
        downgraded: true,
        display: None,
        diagnostic: Some("fixture\n原图：forged".into()),
    }
}
#[test]
fn information_distinguishes_hdr_original_sdr_preview_and_downgrade_without_raw_lines() {
    let mut source = source();
    let text = describe(&source);
    assert!(text.starts_with(&message(
        "capture-info-summary",
        &[
            ("original", source.original_label().into()),
            ("downgraded", tr("common-yes").into()),
            ("reason", tr("capture-info-hdr-failed").into()),
        ]
    )));
    assert!(text.ends_with(&message(
        "capture-info-diagnostic",
        &[("diagnostic", "fixture 原图：forged".into()),]
    )));
    assert!(!text.contains("\n原图：forged"));
    source.original = Some("C:/not-opened/hdr.png".into());
    source.downgraded = false;
    source.reason = None;
    source.display = Some(crate::capture_display::DisplayInfo {
        hdr_enabled: true,
        sdr_white_nits: 203.0,
        minimum_nits: 0.01,
        maximum_nits: 1000.0,
        configuration_known: false,
        white_level_known: false,
    });
    let text = describe(&source);
    assert!(text.starts_with(&message(
        "capture-info-summary",
        &[
            ("original", source.original_label().into()),
            ("downgraded", tr("common-no").into()),
            ("reason", reason(None).into()),
        ]
    )));
    assert!(text.contains(&message(
        "capture-info-display",
        &[
            ("hdr", tr("capture-info-unconfirmed").into()),
            ("white", "203.0".into()),
            ("fallback", tr("capture-info-fallback").into()),
            ("minimum", "0.010".into()),
            ("maximum", "1000.0".into()),
        ]
    )));
}
#[test]
fn diagnostic_text_is_bounded_and_all_capture_reasons_have_labels() {
    assert_eq!(single_line(&"中".repeat(1025)).chars().count(), 1025);
    assert!(single_line(&"中".repeat(1025)).ends_with('…'));
    assert_eq!(single_line("a\0\r\n\tb\u{2028}\u{2029}"), "a    b  ");
    for value in [
        Reason::ExplicitSdr,
        Reason::Workload,
        Reason::CrossMonitor,
        Reason::DisplayUnknown,
        Reason::DisplaySdr,
        Reason::UnsupportedOs,
        Reason::HdrFailed,
        Reason::AutoSdr,
    ] {
        assert_ne!(reason(Some(value)), "—");
    }
}
#[test]
fn native_menu_disables_information_for_non_capture_documents() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let state = State::new(None, true).unwrap();
        let menu = CreatePopupMenu().unwrap();
        append(menu, &state).unwrap();
        assert_ne!(
            GetMenuState(menu, COMMAND as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        DestroyMenu(menu).unwrap();
        state.canvas.borrow_mut().attach_capture(Some(source()));
        let menu = CreatePopupMenu().unwrap();
        append(menu, &state).unwrap();
        assert_eq!(
            GetMenuState(menu, COMMAND as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        DestroyMenu(menu).unwrap();
        state.canvas.borrow_mut().editor = crate::editor::Editor::default();
        let menu = CreatePopupMenu().unwrap();
        append(menu, &state).unwrap();
        assert_ne!(
            GetMenuState(menu, COMMAND as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        DestroyMenu(menu).unwrap();
    }
}
