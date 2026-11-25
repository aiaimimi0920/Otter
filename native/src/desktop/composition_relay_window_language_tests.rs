//! Language-only pumps preserve live business state and the owned inspector's reading position.
use super::{observation_tests::*, *};
use crate::{
    live_relay_control_runtime::interaction::Queue,
    live_relay_viewer_trigger_queue::Phase,
    ui_text::{self, DisplayLanguage},
};
use windows::Win32::UI::{
    Controls::{EM_GETFIRSTVISIBLELINE, EM_GETSEL, EM_LINESCROLL, EM_SETSEL},
    Shell::SetWindowSubclass,
};
struct LanguageReset(DisplayLanguage);
impl LanguageReset {
    fn english() -> Self {
        let reset = Self(ui_text::current_language());
        ui_text::set_language(DisplayLanguage::English);
        reset
    }
}
impl Drop for LanguageReset {
    fn drop(&mut self) {
        ui_text::set_language(self.0);
    }
}
unsafe fn menu_text(hwnd: HWND, command: usize) -> String {
    let mut buffer = [0u16; 256];
    let length = GetMenuStringW(
        GetSystemMenu(hwnd, false),
        command as u32,
        Some(&mut buffer),
        MF_BYCOMMAND,
    );
    String::from_utf16(&buffer[..length as usize]).unwrap()
}
unsafe fn assert_labels(window: &RelayWindow, trigger_key: &str) {
    assert_eq!(
        text(window.hwnd),
        ui_text::text("integration-remote-viewer")
    );
    for (command, key) in [
        (input::ACQUIRE, "integration-acquire-remote-control"),
        (input::RELEASE, "integration-release-remote-control"),
        (observation::OPEN, "integration-view-observations"),
        (trigger::OPEN, trigger_key),
    ] {
        assert_eq!(menu_text(window.hwnd, command), ui_text::text(key));
    }
}
#[test]
fn language_refresh_relabels_same_viewer_and_inspector_without_business_events() {
    let _language = LanguageReset::english();
    let _com = Apartment::new().unwrap();
    unsafe {
        let (window, media, permit) = create();
        let snapshot = business(7);
        media.publish_business(&permit, snapshot.clone()).unwrap();
        let queue = Arc::new(Queue::default());
        window.bind_input(queue.clone());
        queue
            .triggers
            .configure(Some(("surface:real".into(), "attachment:real".into())));
        window.refresh().unwrap();
        let (panel, edit) = open(&window);
        let command = r#"{"bindingId":"b","enabled":true,"target":{"surfaceInstanceId":"surface:real","surfaceAttachmentId":"attachment:real","surfaceNodeId":"n","surfaceEvent":"click","surfaceAction":"x"},"condition":{"conditionId":"c","revision":1,"observationId":"o","operator":"equals","operand":true,"stableForMs":0,"risingEdge":false,"rearm":false,"minimumConfidence":"exact"}}"#;
        assert!(trigger::submit_test(window.hwnd, command));
        let status = queue.status();
        SetWindowPos(
            panel,
            None,
            -10000,
            -10000,
            400,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(2)), Some(LPARAM(7)));
        SendMessageW(edit, EM_LINESCROLL, Some(WPARAM(0)), Some(LPARAM(2)));
        let first = SendMessageW(edit, EM_GETFIRSTVISIBLELINE, None, None);
        assert!(first.0 > 0);
        for language in [DisplayLanguage::SimplifiedChinese, DisplayLanguage::English] {
            ui_text::set_language(language);
            window.refresh().unwrap();
            assert_eq!(window.observation_test(), Some((panel, edit)));
            assert_labels(&window, "integration-trigger-pending");
            assert_eq!(text(panel), ui_text::text("integration-observations-title"));
            assert!(text(edit).contains(&ui_text::format(
                "integration-observation-value",
                &[("value", "7".into())]
            )));
            let selection = SendMessageW(edit, EM_GETSEL, None, None).0 as usize;
            assert_eq!((selection & 0xffff, selection >> 16), (2, 7));
            assert_eq!(
                SendMessageW(edit, EM_GETFIRSTVISIBLELINE, None, None),
                first
            );
            assert_eq!(queue.status(), status);
            assert_eq!(queue.triggers.phase(), Phase::Pending);
            let current = window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .business_snapshot()
                .unwrap();
            assert!(Arc::ptr_eq(&current, &snapshot));
        }
    }
}
#[test]
fn language_refresh_relabels_disabled_menus_without_an_input_queue() {
    let _language = LanguageReset::english();
    let _com = Apartment::new().unwrap();
    unsafe {
        let (window, _, _) = create();
        window.refresh().unwrap();
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        window.refresh().unwrap();
        assert_labels(&window, "integration-configure-trigger");
        let menu = GetSystemMenu(window.hwnd, false);
        for command in [input::ACQUIRE, input::RELEASE, trigger::OPEN] {
            assert_ne!(
                GetMenuState(menu, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
        }
    }
}
#[test]
fn language_caption_reentrancy_never_restores_a_closed_inspector() {
    let _language = LanguageReset::english();
    let _com = Apartment::new().unwrap();
    unsafe {
        let (window, media, permit) = create();
        media.publish_business(&permit, business(7)).unwrap();
        window.refresh().unwrap();
        let (panel, edit) = open(&window);
        assert!(SetWindowSubclass(panel, Some(close_parent), 1, window.hwnd.0 as usize).as_bool());
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        window.refresh().unwrap();
        assert!(!window.is_alive());
        assert!(window.state.observation.borrow().is_none());
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(!IsWindow(Some(edit)).as_bool());
        assert!(media.validate(&permit).is_ok());
    }
}
