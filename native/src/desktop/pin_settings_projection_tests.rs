//! Inline screenshot rows and the original request lifecycle: tabs neither enqueue networking nor cancel; invalid sources/disabled buttons cannot commit.
use super::*;
use crate::desktop::device_authorization_ui::{self as auth, Request};
unsafe fn rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect).unwrap();
    rect
}
#[test]
fn inline_tabs_keep_compact_rows_and_inflight_request_while_switching() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        let width = field(panel, settings::WIDTH);
        assert_eq!(rect(field(panel, 202)).top, rect(field(panel, 203)).top);
        for ids in [
            &[204, 205, 206][..],
            &[221, 222, 223],
            &[219, 208, 209, 218],
            &[217, 201],
        ] {
            let top = rect(field(panel, ids[0])).top;
            assert!(ids.iter().all(|id| rect(field(panel, *id)).top == top));
        }
        assert!(IsWindowEnabled(field(panel, 218)).as_bool());
        for id in [222, 223] {
            assert!(!IsWindowEnabled(field(panel, id)).as_bool());
        }
        f.state.authorization.borrow_mut().request = Some(Request::ProjectionTargets);
        settings::refresh(&f.state);
        for id in [203, 202, 203] {
            SendMessageW(field(panel, id), BM_CLICK, None, None);
            dispatch(&f);
            assert!(matches!(
                f.state.authorization.borrow().request,
                Some(Request::ProjectionTargets)
            ));
            assert_eq!(field(panel, settings::WIDTH), width);
        }
        assert_eq!(rect(field(panel, 220)).top, rect(field(panel, 207)).top);
        assert_eq!(rect(field(panel, 219)).top, rect(field(panel, 210)).top);
        assert_eq!(rect(field(panel, 219)).top, rect(field(panel, 211)).top);
        assert!(!IsWindowEnabled(field(panel, 207)).as_bool());
        assert!(!IsWindowEnabled(field(panel, 210)).as_bool());
        assert!(!IsWindowEnabled(field(panel, 211)).as_bool());
        assert!(f.state.error.borrow().is_none());
    }
}
#[test]
fn old_tab_notification_cannot_apply_after_origin_changes() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        SendMessageW(field(panel, 203), BM_CLICK, None, None);
        f.state.authorization.borrow_mut().origin = Some("https://other.example".into());
        dispatch(&f);
        // A rejected stale message must not switch to the receiving page; device buttons remain.
        assert!(IsWindowVisible(field(panel, 208)).as_bool());
        assert!(f.state.authorization.borrow().request.is_none());
    }
}
#[test]
fn projection_inline_icons_use_supported_baseline_geometry() {
    for svg in [
        include_str!("../../assets/edit-strip/SettingsOneWay.svg"),
        include_str!("../../assets/edit-strip/SettingsOneWayReceive.svg"),
        include_str!("../../assets/edit-strip/SettingsTwoWay.svg"),
        include_str!("../../assets/edit-strip/SettingsImport.svg"),
        include_str!("../../assets/edit-strip/SettingsExport.svg"),
        include_str!("../../assets/edit-strip/SettingsQr.svg"),
        include_str!("../../assets/edit-strip/SettingsLink.svg"),
        include_str!("../../assets/edit-strip/SettingsSource.svg"),
        include_str!("../../assets/edit-strip/SettingsCopy.svg"),
        include_str!("../../assets/edit-strip/SettingsUnlink.svg"),
        include_str!("../../assets/edit-strip/SettingsDevices.svg"),
        include_str!("../../assets/edit-strip/SettingsGroups.svg"),
        include_str!("../../assets/edit-strip/SettingsFriends.svg"),
    ] {
        neuro_mini_ui::Icon::svg(svg).validate().unwrap();
    }
}
#[test]
fn device_and_group_buttons_filter_only_commands_not_the_shared_sender_owner() {
    unsafe {
        let f = Fixture::new();
        open(&f);
        f.state.authorization.borrow_mut().origin = Some("https://loom.example".into());
        f.state.authorization.borrow_mut().targets.directory = Some(std::sync::Arc::new(
            crate::projection_delivery::targets::Directory::parse(
                &serde_json::to_vec(&crate::projection_delivery::targets::tests::body()).unwrap(),
                "https://loom.example",
            )
            .unwrap(),
        ));
        let devices = auth::settings::items(auth::settings::Section::Devices, &f.state).unwrap();
        assert!(devices.iter().any(|i| i.command == auth::sender::BASE));
        let groups = auth::settings::items(auth::settings::Section::Groups, &f.state).unwrap();
        assert!(!groups.iter().any(|i| i.command == auth::sender::BASE));
        assert!(groups.iter().any(|i| i.command == auth::targets::REFRESH));
        assert!(f.state.authorization.borrow().request.is_none());
    }
}

#[test]
fn two_way_can_be_selected_before_projection_and_survives_sidebar_reopen() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        assert!(IsWindowEnabled(field(panel, 206)).as_bool());
        SendMessageW(field(panel, 206), BM_CLICK, None, None);
        dispatch(&f);
        assert_eq!(
            auth::editing::mode(&f.state),
            crate::projection_edit::Mode::TwoWay
        );
        assert!(f.state.projection_edit.borrow().is_none());
        assert!(f.state.authorization.borrow().request.is_none());
        settings::close(&f.state);
        settings::toggle(f.owner.0, &f.state).unwrap();
        assert_eq!(
            auth::editing::mode(&f.state),
            crate::projection_edit::Mode::TwoWay
        );
        SendMessageW(field(panel, 205), BM_CLICK, None, None);
        dispatch(&f);
        assert_eq!(
            auth::editing::mode(&f.state),
            crate::projection_edit::Mode::OneWay
        );
    }
}
