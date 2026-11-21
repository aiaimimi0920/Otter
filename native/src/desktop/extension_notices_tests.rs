//! Otter 的通知预算/撤权真实消费；主题的滚动与 HWND 回收不接管业务 scope。
use super::*;
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};
#[test]
fn application_consumer_keeps_newest_eight_and_revocation_reflows_the_same_stack() {
    unsafe {
        let _apartment = super::super::Apartment::new().unwrap();
        let owner = super::super::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Otter notice fixture"),
                WS_POPUP,
                100,
                200,
                400,
                180,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let state = State::new(None, true).unwrap();
        state.extensions.borrow_mut().snapshot = Some(std::sync::Arc::new(
            crate::loom_extension_snapshot::Snapshot::parse(
                &serde_json::to_vec(&crate::extension_visual::tests::catalog_value()).unwrap(),
            )
            .unwrap(),
        ));
        show(owner.0, &state, None, "原宿主消息", "示例").unwrap();
        let removed = state.extension_notices.borrow().entries[0]
            .notice
            .window()
            .unwrap();
        for _ in 0..8 {
            show(owner.0, &state, Some("scope:ocr"), "识别完成", "示例结果").unwrap();
        }
        let root = state
            .extension_notices
            .borrow()
            .stack
            .as_ref()
            .unwrap()
            .window()
            .unwrap();
        assert!(!IsWindow(Some(removed)).as_bool());
        assert_eq!(state.extension_notices.borrow().entries.len(), 8);
        let mut data = crate::extension_visual::tests::catalog_value();
        data["plugins"][0]["trustStatus"] = "unsigned_developer".into();
        state.extensions.borrow_mut().snapshot = Some(std::sync::Arc::new(
            crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&data).unwrap())
                .unwrap(),
        ));
        sync(&state).unwrap();
        assert_eq!(state.extension_notices.borrow().entries.len(), 8);
        let oldest = state.extension_notices.borrow().entries[7]
            .notice
            .window()
            .unwrap();
        let newest = state.extension_notices.borrow().entries[0]
            .notice
            .window()
            .unwrap();
        let mut a = windows::Win32::Foundation::RECT::default();
        let mut b = a;
        GetWindowRect(oldest, &mut a).unwrap();
        GetWindowRect(newest, &mut b).unwrap();
        assert!(a.top > b.top);
        SendMessageW(
            newest,
            WM_KEYDOWN,
            Some(windows::Win32::Foundation::WPARAM(13)),
            None,
        );
        sync(&state).unwrap();
        assert_eq!(state.extension_notices.borrow().entries.len(), 7);
        assert_eq!(
            state
                .extension_notices
                .borrow()
                .stack
                .as_ref()
                .unwrap()
                .window(),
            Some(root)
        );
        data["plugins"][0]["trustStatus"] = "revoked".into();
        state.extensions.borrow_mut().snapshot = Some(std::sync::Arc::new(
            crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&data).unwrap())
                .unwrap(),
        ));
        sync(&state).unwrap();
        assert!(state.extension_notices.borrow().entries.is_empty());
        assert!(!IsWindow(Some(oldest)).as_bool());
        assert!(!IsWindowVisible(root).as_bool());
        show(owner.0, &state, None, "仍可显示本地错误", "示例").unwrap();
        state.extensions.borrow_mut().snapshot = None;
        sync(&state).unwrap();
        assert_eq!(state.extension_notices.borrow().entries.len(), 1);
        let card = state.extension_notices.borrow().entries[0]
            .notice
            .window()
            .unwrap();
        drop(owner);
        assert!(!IsWindow(Some(card)).as_bool());
        sync(&state).unwrap();
        assert!(state.extension_notices.borrow().entries.is_empty());
    }
}
