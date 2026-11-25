//! Inline screenshot rows and the original request lifecycle: tabs neither enqueue networking nor cancel; invalid sources/disabled buttons cannot commit.
use super::*;
use crate::desktop::device_authorization_ui::{self as auth, Request};
use crate::ui_text::{self, DisplayLanguage};
pub(super) struct LanguageReset(pub DisplayLanguage);
impl Drop for LanguageReset {
    fn drop(&mut self) {
        ui_text::set_language(self.0);
    }
}
pub(super) unsafe fn caption(hwnd: HWND) -> String {
    let mut text = [0u16; 512];
    let length = GetWindowTextW(hwnd, &mut text) as usize;
    String::from_utf16_lossy(&text[..length])
}
pub(super) unsafe fn tooltip(panel: HWND, child: HWND) -> String {
    use windows::Win32::{System::Threading::GetCurrentThreadId, UI::Controls::*};
    let mut target = (panel, HWND::default());
    unsafe extern "system" fn find(hwnd: HWND, data: LPARAM) -> windows::core::BOOL {
        let target = &mut *(data.0 as *mut (HWND, HWND));
        let mut class = [0u16; 32];
        let length = GetClassNameW(hwnd, &mut class) as usize;
        if GetWindow(hwnd, GW_OWNER).ok() == Some(target.0)
            && String::from_utf16_lossy(&class[..length]) == "tooltips_class32"
        {
            target.1 = hwnd;
        }
        true.into()
    }
    assert!(EnumThreadWindows(
        GetCurrentThreadId(),
        Some(find),
        LPARAM(&mut target as *mut _ as isize),
    )
    .as_bool());
    assert!(!target.1.is_invalid());
    let mut text = [0u16; 512];
    let mut info = TTTOOLINFOW {
        cbSize: std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32,
        hwnd: panel,
        uId: child.0 as usize,
        lpszText: windows::core::PWSTR(text.as_mut_ptr()),
        ..Default::default()
    };
    SendMessageW(
        target.1,
        TTM_GETTEXTW,
        Some(WPARAM(text.len())),
        Some(LPARAM(&mut info as *mut _ as isize)),
    );
    let length = text.iter().position(|c| *c == 0).unwrap();
    String::from_utf16_lossy(&text[..length])
}
unsafe fn rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect).unwrap();
    rect
}
#[test]
fn inline_tabs_keep_compact_rows_and_inflight_request_while_switching() {
    let _reset = LanguageReset(ui_text::current_language());
    for language in DisplayLanguage::ALL {
        ui_text::set_language(language);
        unsafe {
            let f = Fixture::new();
            let panel = open(&f);
            let width = field(panel, settings::WIDTH);
            let controls: Vec<_> = [201, 202, 203, 204, 205, 206, 217, 218, 219, 221, 222, 223]
                .into_iter()
                .map(|id| (id, field(panel, id)))
                .collect();
            for display in DisplayLanguage::ALL {
                ui_text::set_language(display);
                settings::refresh(&f.state);
                for &(id, hwnd) in &controls {
                    assert_eq!(field(panel, id), hwnd);
                }
                for (id, key) in [
                    (201, "pin-connect"),
                    (202, "pin-send"),
                    (203, "pin-receive"),
                ] {
                    let expected = ui_text::text(key);
                    assert_eq!(caption(field(panel, id)), expected);
                    assert_eq!(tooltip(panel, field(panel, id)), expected);
                }
                assert_eq!(rect(field(panel, 202)).top, rect(field(panel, 203)).top);
                for ids in [
                    &[204, 205, 206][..],
                    &[221, 222, 223],
                    &[219, 208, 209, 218],
                    &[217, 201],
                ] {
                    let top = rect(field(panel, ids[0])).top;
                    assert!(
                        ids.iter().all(|id| rect(field(panel, *id)).top == top),
                        "{display:?}: {ids:?}"
                    );
                }
            }
            ui_text::set_language(language);
            settings::refresh(&f.state);
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
}
struct CaptionHook {
    owner: HWND,
    state: *const crate::desktop::State,
    mode: u8,
    fired: std::cell::Cell<bool>,
}
unsafe extern "system" fn caption_hook(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let hook = &*(data as *const CaptionHook);
    if message == WM_SETTEXT && !hook.fired.replace(true) {
        match hook.mode {
            0 => {
                SendMessageW(hook.owner, WM_NULL, None, None);
            }
            1 => return LRESULT(0),
            _ => settings::close(&*hook.state),
        }
    }
    windows::Win32::UI::Shell::DefSubclassProc(hwnd, message, wp, lp)
}
#[test]
fn caption_refresh_reentry_and_failed_writes_preserve_panel_and_numeric_drafts() {
    use windows::Win32::UI::Shell::{RemoveWindowSubclass, SetWindowSubclass};
    let _reset = LanguageReset(ui_text::current_language());
    ui_text::set_language(DisplayLanguage::English);
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        let connect = field(panel, 201);
        let width = field(panel, settings::WIDTH);
        SetWindowTextW(width, windows::core::w!("456")).unwrap();
        for (mode, language) in [
            (0, DisplayLanguage::SimplifiedChinese),
            (1, DisplayLanguage::English),
            (2, DisplayLanguage::SimplifiedChinese),
        ] {
            let hook = CaptionHook {
                owner: f.owner.0,
                state: &*f.state,
                mode,
                fired: std::cell::Cell::new(false),
            };
            SetWindowSubclass(connect, Some(caption_hook), 1, &hook as *const _ as usize)
                .ok()
                .unwrap();
            ui_text::set_language(language);
            settings::refresh(&f.state);
            assert!(hook.fired.get());
            RemoveWindowSubclass(connect, Some(caption_hook), 1)
                .ok()
                .unwrap();
            assert_eq!(field(panel, settings::WIDTH), width);
            assert_eq!(caption(width), "456");
            if mode == 2 {
                assert!(!settings::is_open(&f.state));
                assert!(!IsWindowVisible(panel).as_bool());
            } else {
                assert!(settings::is_open(&f.state));
                assert!(IsWindowVisible(panel).as_bool());
                // A failed write is not cached as success; the next refresh retries it.
                if mode == 1 {
                    settings::refresh(&f.state);
                }
                assert_eq!(field(panel, 201), connect);
                assert_eq!(caption(connect), ui_text::text("pin-connect"));
                assert_eq!(caption(field(panel, 203)), ui_text::text("pin-receive"));
            }
        }
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
