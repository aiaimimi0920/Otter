//! 真实分享 HWND/EDIT/按钮与生命周期，全部窗口归当前隔离测试所有。
use super::*;
use crate::projection_account::{scene::Binding, tests::scene_tests::source_fixture};
struct Fixture {
    popup: Window,
    view: Rc<RefCell<View>>,
    _com: crate::desktop::Apartment,
}
impl Fixture {
    unsafe fn new(receive: bool, qr: bool) -> Self {
        let com = crate::desktop::Apartment::new().unwrap();
        let view = Rc::new(RefCell::new(View::default()));
        view.borrow_mut().origin = Some("https://loom.example".into());
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            400,
            300,
            400,
            240,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let model = Rc::new(Model {
            hwnd: Cell::new(None),
            owner,
            view: Rc::downgrade(&view),
            unit: UnitId::parse("unit-1").unwrap(),
            origin: "https://loom.example".into(),
            receive,
            public: Cell::new(true),
            envelope: RefCell::new(None),
            source_loom_id: RefCell::new(String::new()),
            account_envelope: RefCell::new(None),
            account_import: Cell::new(false),
            create_pending: Cell::new(false),
            target: crate::desktop::popup_menu::target::MenuTarget::capture(
                &crate::desktop::canvas::Canvas::new(None).unwrap(),
            ),
            qr: Cell::new(qr),
            input_epoch: Cell::new(0),
            file_dialog_active: Cell::new(false),
            decode_cancel: RefCell::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
            code: RefCell::new(None),
            link: RefCell::new(String::new()),
            inputs: RefCell::new(Default::default()),
            status: RefCell::new(String::new()),
        });
        create(&model).unwrap();
        Self {
            popup: Window { model },
            view,
            _com: com,
        }
    }
    fn hwnd(&self) -> HWND {
        self.popup.model.hwnd.get().unwrap()
    }
    unsafe fn command(&self, id: usize) {
        SendMessageW(self.hwnd(), WM_COMMAND, Some(WPARAM(id)), Some(LPARAM(0)));
    }
    unsafe fn text(&self) -> String {
        let mut buffer = vec![0; MAX_TEXT + 1];
        let n = GetDlgItemTextW(self.hwnd(), 101, &mut buffer) as usize;
        String::from_utf16_lossy(&buffer[..n])
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.popup.model.owner);
        }
    }
}
fn invitation() -> Binding {
    source_fixture("https://loom.example")
}
#[test]
fn account_receive_panel_exposes_unlink_for_existing_binding() {
    unsafe {
        let f = Fixture::new(true, false);
        let scope = serde_json::from_value(serde_json::json!("a".repeat(64))).unwrap();
        let (_, binding) = crate::projection_account::tests::scene_tests::fixture(scope, "unit-1");
        let mut document = serde_json::to_value(binding).unwrap();
        document["loomOrigin"] = serde_json::json!("https://loom.example");
        let binding = Binding::decode(&document.to_string(), "unit-1", scope).unwrap();
        f.view.borrow_mut().account.bindings = vec![binding.clone()];
        f.popup.refresh();
        assert!(IsWindowVisible(GetDlgItem(Some(f.hwnd()), 108).unwrap()).as_bool());
        f.command(108);
        assert!(matches!(f.view.borrow_mut().request.take(),
            Some(Request::AccountProjection(request))
                if matches!(*request, crate::desktop::device_authorization_ui::account::Request::Stop(ref value)
                    if value == &binding)));
        f.command(109);
        f.popup.refresh();
        assert!(!IsWindowVisible(GetDlgItem(Some(f.hwnd()), 108).unwrap()).as_bool());
        f.command(108);
        assert!(f.view.borrow().request.is_none());
        f.command(110);
        f.popup.refresh();
        assert!(IsWindowVisible(GetDlgItem(Some(f.hwnd()), 108).unwrap()).as_bool());
    }
}
#[test]
fn share_dialog_tabs_preserve_input_and_parse_never_queues_network() {
    unsafe {
        let f = Fixture::new(true, false);
        let binding = invitation();
        let text = binding.envelope().link().unwrap();
        // 多行 EDIT 的 WM_SETTEXT 不产生 EN_CHANGE；真实键入/粘贴通过替换选择产生通知。
        let input = HSTRING::from(&text);
        SendMessageW(
            GetDlgItem(Some(f.hwnd()), 101).unwrap(),
            0x00c2,
            Some(WPARAM(1)),
            Some(LPARAM(input.as_ptr() as isize)),
        );
        assert!(f.popup.model.account_envelope.borrow().is_some());
        assert!(f.view.borrow().request.is_none());
        f.command(105);
        assert_eq!(f.text(), text);
        assert!(f.popup.model.account_envelope.borrow().is_some());
        f.command(104);
        assert_eq!(f.text(), text);
        assert!(f.popup.model.account_envelope.borrow().is_some());
        assert!(IsWindowVisible(GetDlgItem(Some(f.hwnd()), 103).unwrap()).as_bool());
        assert!(!IsWindowVisible(GetDlgItem(Some(f.hwnd()), 101).unwrap()).as_bool());
        assert!(f.view.borrow().request.is_none());
        f.command(105);
        assert_eq!(f.text(), text);
        assert!(
            windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                GetDlgItem(Some(f.hwnd()), 1).unwrap()
            )
            .as_bool()
        );
    }
}
#[test]
fn share_dialog_removed_or_wrong_origin_binding_clears_output_and_blocks_copy() {
    unsafe {
        let f = Fixture::new(false, false);
        f.view.borrow_mut().account.bindings.push(invitation());
        f.popup.refresh();
        assert!(f.popup.model.output_current());
        assert!(!f.text().is_empty());
        f.view.borrow_mut().account.bindings.clear();
        assert!(!f.popup.model.output_current());
        assert!(copy(&f.popup.model).is_err());
        f.popup.refresh();
        assert!(f.text().is_empty());
        assert!(f.popup.model.link.borrow().is_empty());
        f.view.borrow_mut().origin = Some("https://other.example".into());
        f.popup.refresh();
        assert!(f.popup.model.hwnd.get().is_none());
    }
}
#[test]
fn share_views_reuse_window_extend_beyond_pin_follow_move_and_cancel_decoder() {
    unsafe {
        let f = Fixture::new(false, true);
        let binding = invitation();
        let expected_link = binding.envelope().link().unwrap();
        f.view.borrow_mut().account.bindings.push(binding);
        f.popup.refresh();
        let hwnd = f.hwnd();
        assert!(!IsWindowVisible(GetDlgItem(Some(hwnd), 101).unwrap()).as_bool());
        assert!(IsWindowVisible(GetDlgItem(Some(hwnd), 106).unwrap()).as_bool());
        assert_eq!(f.text(), expected_link);
        let mut owner = RECT::default();
        let mut popup = RECT::default();
        GetWindowRect(f.popup.model.owner, &mut owner).unwrap();
        GetWindowRect(hwnd, &mut popup).unwrap();
        assert!(popup.bottom - popup.top > owner.bottom - owner.top);
        assert_eq!(
            GetWindowLongW(hwnd, GWL_STYLE) as u32 & (WS_HSCROLL.0 | WS_VSCROLL.0),
            0
        );
        SetWindowPos(
            f.popup.model.owner,
            None,
            owner.left + 24,
            owner.top + 16,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        place(&f.popup.model);
        let mut moved = RECT::default();
        GetWindowRect(hwnd, &mut moved).unwrap();
        assert_eq!((moved.left - popup.left, moved.top - popup.top), (24, 16));
        f.command(105);
        assert_eq!(f.hwnd(), hwnd);
        assert!(IsWindowVisible(GetDlgItem(Some(hwnd), 101).unwrap()).as_bool());
        assert!(!IsWindowVisible(GetDlgItem(Some(hwnd), 106).unwrap()).as_bool());
        assert_eq!(
            GetWindowLongW(GetDlgItem(Some(hwnd), 101).unwrap(), GWL_STYLE) as u32
                & ES_MULTILINE as u32,
            0
        );
        assert_eq!(f.text(), expected_link);
        assert!(f.view.borrow().request.is_none());
        for id in [101, 104, 105, 109, 110, 106, 1] {
            ValidateRect(Some(GetDlgItem(Some(hwnd), id).unwrap()), None)
                .ok()
                .unwrap();
        }
        ValidateRect(Some(hwnd), None).ok().unwrap();
        for _ in 0..8 {
            f.popup.refresh();
        }
        assert!(!GetUpdateRect(hwnd, None, false).as_bool());
        for id in [101, 104, 105, 109, 110, 106, 1] {
            assert!(!GetUpdateRect(GetDlgItem(Some(hwnd), id).unwrap(), None, false).as_bool());
        }
        let signal = Arc::new(AtomicBool::new(false));
        *f.popup.model.decode_cancel.borrow_mut() = Some(signal.clone());
        DestroyWindow(f.popup.model.owner).unwrap();
        assert!(f.popup.model.hwnd.get().is_none());
        assert!(signal.load(Ordering::Acquire));
        assert!(f.popup.model.cancel.load(Ordering::Acquire));
        assert_eq!(Rc::strong_count(&f.popup.model), 1);
    }
}
#[test]
fn share_empty_output_refresh_does_not_self_invalidate_idle_window() {
    unsafe {
        let f = Fixture::new(false, false);
        f.popup.refresh();
        ValidateRect(Some(f.hwnd()), None).ok().unwrap();
        for _ in 0..8 {
            f.popup.refresh();
        }
        assert!(!GetUpdateRect(f.hwnd(), None, false).as_bool());
    }
}

#[test]
fn account_share_retries_explicitly_and_idle_refresh_does_not_self_invalidate() {
    unsafe {
        let f = Fixture::new(false, false);
        assert!(GetDlgItem(Some(f.hwnd()), 107).is_err());
        f.command(108);
        assert!(matches!(f.view.borrow_mut().request.take(),
            Some(Request::AccountProjection(request))
                if matches!(*request, crate::desktop::device_authorization_ui::account::Request::Create(_))));
        f.view.borrow_mut().account.status = "创建失败".into();
        f.popup.refresh();
        ValidateRect(Some(f.hwnd()), None).ok().unwrap();
        for _ in 0..8 {
            f.popup.refresh();
        }
        assert!(!GetUpdateRect(f.hwnd(), None, false).as_bool());
        f.command(108);
        assert!(matches!(f.view.borrow_mut().request.take(),
            Some(Request::AccountProjection(request))
                if matches!(*request, crate::desktop::device_authorization_ui::account::Request::Create(_))));
    }
}
#[test]
fn account_link_submit_queues_only_v2_and_rejects_other_protocols() {
    unsafe {
        let f = Fixture::new(true, false);
        let binding = invitation();
        f.popup.model.parsed(&binding.envelope().link().unwrap());
        assert!(f.view.borrow().request.is_none());
        f.command(1);
        assert!(matches!(f.view.borrow_mut().request.take(),
            Some(Request::AccountProjection(request))
                if matches!(*request, crate::desktop::device_authorization_ui::account::Request::Import { .. })));
        f.popup.model.account_import.set(false);
        let old = serde_json::to_string(binding.envelope())
            .unwrap()
            .replace("neuro.qr-projection.v2", "neuro.qr-projection.v1");
        f.popup.model.parsed(&old);
        assert!(f.popup.model.account_envelope.borrow().is_none());
        assert!(f.popup.model.submit().is_err());
        assert!(f.view.borrow().request.is_none());
    }
}

#[test]
fn account_stop_uses_only_the_current_loom_binding() {
    unsafe {
        let f = Fixture::new(false, false);
        let other = source_fixture("https://other.example");
        f.view.borrow_mut().account.bindings.push(other);
        f.command(108);
        assert!(matches!(f.view.borrow_mut().request.take(),
            Some(Request::AccountProjection(request))
                if matches!(*request, crate::desktop::device_authorization_ui::account::Request::Create(_))));
        f.view.borrow_mut().account.bindings.push(invitation());
        f.command(108);
        assert!(matches!(f.view.borrow_mut().request.take(),
            Some(Request::AccountProjection(request))
                if matches!(&*request, crate::desktop::device_authorization_ui::account::Request::Stop(binding)
                    if binding.origin() == "https://loom.example")));
    }
}

#[test]
fn background_work_allows_scope_navigation_and_preserves_receiver_input() {
    unsafe {
        let f = Fixture::new(true, false);
        let link = invitation().envelope().link().unwrap();
        f.popup.model.parsed(&link);
        for kind in 0..4 {
            {
                let mut v = f.view.borrow_mut();
                v.sender.busy = kind == 0;
                v.account.busy = kind == 1;
                v.projection.busy = kind == 2;
                v.editing.busy = kind == 3;
            }
            f.command(109);
            assert!(!f.popup.model.public.get());
            assert!(f.text().is_empty());
            f.command(110);
            assert!(f.popup.model.public.get());
            assert_eq!(f.text(), link);
            assert!(f.popup.model.account_envelope.borrow().is_some());
            assert!(f.view.borrow().request.is_none());
        }
    }
}

#[test]
fn first_show_has_exactly_one_input_view_without_needing_a_tab_click() {
    unsafe {
        for receive in [false, true] {
            for qr in [false, true] {
                let f = Fixture::new(receive, qr);
                let visible =
                    |id| IsWindowVisible(GetDlgItem(Some(f.hwnd()), id).unwrap()).as_bool();
                assert_eq!(visible(101), !qr, "link visibility on first show");
                assert_eq!(
                    visible(103),
                    receive && qr,
                    "image picker visibility on first show"
                );
                assert!(!visible(106), "empty QR must not cover the receiver");
                assert!(!visible(102), "empty status must not overlap input");
                f.command(if qr { 105 } else { 104 });
                assert_eq!(visible(101), qr);
                assert_eq!(visible(103), receive && !qr);
            }
        }
    }
}

#[test]
fn background_sync_and_message_pump_do_not_repaint_or_disable_share_navigation() {
    unsafe {
        let f = Fixture::new(false, true);
        f.view.borrow_mut().account.bindings.push(invitation());
        f.popup.refresh();
        let settle = || {
            for _ in 0..256 {
                let mut message = MSG::default();
                if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    return;
                }
                if !f.popup.translate(&message) {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            panic!("share message queue did not become idle");
        };
        settle();
        for n in 0..16 {
            {
                let mut v = f.view.borrow_mut();
                v.sender.busy = n % 2 == 0;
                v.editing.busy = n % 2 != 0;
                v.account.busy = n % 3 == 0;
            }
            f.popup.refresh();
            for id in [101, 103, 104, 105, 109, 110, 1] {
                let control = GetDlgItem(Some(f.hwnd()), id).unwrap();
                assert!(IsWindowEnabled(control).as_bool());
                assert!(
                    !GetUpdateRect(control, None, false).as_bool(),
                    "background invalidated {id}"
                );
            }
            assert!(!GetUpdateRect(f.hwnd(), None, false).as_bool());
            settle();
        }
    }
}

#[test]
fn deferred_create_is_cancelled_by_close_or_explicit_action() {
    unsafe {
        for close in [false, true] {
            let f = Fixture::new(false, true);
            f.popup.model.create_pending.set(true);
            f.view.borrow_mut().sender.busy = true;
            f.popup.refresh();
            assert!(f.view.borrow().request.is_none());
            if close {
                SendMessageW(f.hwnd(), WM_CLOSE, None, None);
            } else {
                f.command(108);
            }
            f.view.borrow_mut().sender.busy = false;
            f.popup.refresh();
            assert!(!f.popup.model.create_pending.get());
            assert!(f.view.borrow().request.is_none());
        }
    }
}

#[test]
fn completed_import_releases_navigation_while_unrelated_sync_is_busy() {
    unsafe {
        for public in [false, true] {
            let f = Fixture::new(true, false);
            f.popup.model.public.set(public);
            if public {
                f.popup
                    .model
                    .parsed(&invitation().envelope().link().unwrap());
            }
            f.popup.model.account_import.set(true);
            f.view.borrow_mut().sender.busy = true;
            f.popup.refresh();
            assert!(!f.popup.model.account_import.get());
            for id in [101, 104, 105, 109, 110] {
                assert!(IsWindowEnabled(GetDlgItem(Some(f.hwnd()), id).unwrap()).as_bool());
            }
            assert!(f.view.borrow().request.is_none());
        }
    }
}
