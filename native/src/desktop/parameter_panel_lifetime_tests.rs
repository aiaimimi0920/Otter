use super::*;
#[test]
fn hidden_real_panel_drop_reclaims_children_and_only_its_identity() {
    unsafe {
        let f = Fixture::new();
        let panel = f.panel.handle();
        let children: Vec<_> = f
            .panel
            .ui
            .placements
            .borrow()
            .iter()
            .map(|(h, _)| *h)
            .collect();
        assert_eq!(f.state.parameters_ui.hwnd.get(), panel);
        let Fixture {
            panel: owner,
            host,
            state,
        } = f;
        drop(owner);
        assert!(state.parameters_ui.hwnd.get().is_invalid());
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(children.into_iter().all(|h| !IsWindow(Some(h)).as_bool()));
        assert!(IsWindow(Some(host.0)).as_bool());
    }
}
#[test]
fn group_lock_blocks_edits_but_does_not_trap_close() {
    unsafe {
        let f = Fixture::new();
        f.state.group_locked.set(true);
        f.click("enabled:boolean");
        assert!(f.panel.ui.pending.borrow().is_empty());
        f.tick();
        assert!(!IsWindowEnabled(f.binding("enabled:boolean").hwnd).as_bool());
        SendMessageW(f.panel.handle(), WM_CLOSE, None, None);
        assert!(f.panel.retired());
        assert_eq!(f.value("enabled"), false);
    }
}
#[test]
fn dpi_layout_does_not_destroy_dirty_or_composing_edit() {
    unsafe {
        let f = Fixture::new();
        let edit = f.text("text:edit", "草稿🙂");
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
        f.state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("n", Some(serde_json::json!(6)))
            .unwrap();
        f.tick();
        let rect = RECT {
            left: 10,
            top: 10,
            right: 500,
            bottom: 600,
        };
        SendMessageW(
            f.panel.handle(),
            WM_DPICHANGED,
            Some(WPARAM(144 | 144 << 16)),
            Some(LPARAM(&rect as *const RECT as isize)),
        );
        assert_eq!(f.binding("text:edit").hwnd, edit);
        assert!(keyboard::composing(edit));
        assert_eq!(controls::text(edit).unwrap(), "草稿🙂");
        f.click("reload");
        f.tick();
        assert_eq!(f.binding("text:edit").hwnd, edit);
        assert!(!f.panel.exit_idle());
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
        SendMessageW(edit, WM_KEYUP, Some(WPARAM(VK_RETURN.0 as usize)), None);
    }
}
#[test]
fn parameter_pages_bound_children_and_drop_previous_page_handles() {
    unsafe {
        let parameters = (0..40)
            .map(|i| serde_json::json!({"id":format!("p{i}"),"type":"number","default":i}))
            .collect::<Vec<_>>();
        let f = Fixture::with_parameters(parameters.into());
        assert_eq!(f.panel.ui.model.borrow().pages(), 3);
        assert!(f.panel.ui.placements.borrow().len() < 160);
        let first = f.binding("p0:edit").hwnd;
        f.click("next");
        f.tick();
        assert_eq!(f.panel.ui.page.get(), 1);
        assert!(
            !IsWindow(Some(first)).as_bool()
                || f.panel
                    .ui
                    .bindings
                    .borrow()
                    .iter()
                    .all(|b| b.key != "p0:edit")
        );
        assert!(IsWindow(Some(f.binding("p16:edit").hwnd)).as_bool());
    }
}
#[test]
fn initial_native_panel_title_is_preserved_before_show() {
    unsafe {
        let f = Fixture::new();
        let mut title = [0; 64];
        let length = GetWindowTextW(f.panel.handle(), &mut title) as usize;
        assert_eq!(
            String::from_utf16_lossy(&title[..length]),
            "Otter · 参数（Tab）"
        );
        assert!(!IsWindowVisible(f.panel.handle()).as_bool());
    }
}

#[test]
fn new_panel_inherits_capture_exclusion_before_show() {
    unsafe {
        let f = Fixture::new();
        let Fixture { panel, host, state } = f;
        drop(panel);
        state.capture_protection_requested.set(true);
        let panel = Panel::new(host.0, &state).unwrap();
        let mut affinity = 0;
        GetWindowDisplayAffinity(panel.handle(), &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        assert!(state.capture_protection.get()[2]);
        assert!(!IsWindowVisible(panel.handle()).as_bool());
    }
}
#[test]
fn host_first_destruction_removes_capture_registration_before_handle_reuse() {
    unsafe {
        let f = Fixture::new();
        let panel = f.panel.handle();
        f.state.capture_windows.borrow_mut().push(panel);
        assert_eq!(f.state.parameters_ui.window(), panel);
        DestroyWindow(f.host.0).unwrap();
        assert!(f.panel.retired());
        assert!(f.state.parameters_ui.window().is_invalid());
        assert!(!f.state.capture_windows.borrow().contains(&panel));
    }
}
#[test]
fn wrong_panel_marker_never_returns_a_capture_target_or_destroys_successor() {
    unsafe {
        let f = Fixture::new();
        let hwnd = f.panel.handle();
        let marker = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        assert!(f.state.parameters_ui.window().is_invalid());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, marker);
        assert_eq!(f.state.parameters_ui.window(), hwnd);
    }
}
