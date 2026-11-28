//! Isolated HWNDs on a private desktop, without registering real hotkeys or
//! installing tray icons. Exercise actual moves, stale undo and visibility.
use super::*;
unsafe fn pins() -> Application {
    let mut app = chrome::tests::app();
    for i in 0..3 {
        let image = crate::desktop::image::Image {
            width: 60,
            height: 60,
            pixels: vec![255; 60 * 60 * 4].into(),
        };
        let state = State::new(Some(image), true).unwrap();
        let d = app
            .prepare_window_at(
                state,
                false,
                PinRect {
                    x: 80 + i * 90,
                    y: 70 + i * 30,
                    width: 60,
                    height: 60,
                },
            )
            .unwrap();
        app.workspace.add_units([d.state.unit_id.clone()]).unwrap();
        d.show_content(SW_SHOWNOACTIVATE);
        app.documents.push(d);
    }
    app
}
#[test]
fn private_hwnds_arrange_refuses_locked_member_and_undo_skips_independent_move() {
    crate::desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        let _apt = crate::desktop::Apartment::new().unwrap();
        let mut app = pins();
        let targets = app.documents[1..]
            .iter()
            .map(|d| app.layout_target(&d.state.unit_id).unwrap())
            .collect::<Vec<_>>();
        let before = targets
            .iter()
            .map(|t| frame(t.hwnd).unwrap())
            .collect::<Vec<_>>();
        app.documents[2].state.group_locked.set(true);
        assert!(app
            .arrange_pins(&targets, &targets[1].id, Layout::Top)
            .is_err());
        for (i, t) in targets.iter().enumerate() {
            assert_eq!(frame(t.hwnd).unwrap(), before[i]);
        }
        app.documents[2].state.group_locked.set(false);
        app.arrange_pins(&targets, &targets[1].id, Layout::Top)
            .unwrap();
        for t in &targets {
            assert_eq!(frame(t.hwnd).unwrap().y, before[1].y);
        }
        let custom = PinRect {
            x: 100,
            y: 110,
            ..before[2]
        };
        position(targets[2].hwnd, custom).unwrap();
        assert!(app.undo_layout().is_err());
        assert_eq!(frame(targets[0].hwnd).unwrap(), before[0]);
        assert_eq!(frame(targets[2].hwnd).unwrap(), custom);
        app.shutdown();
    })
}
#[test]
fn shake_only_removes_its_own_reason_when_initiator_closes() {
    crate::desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        let _apt = crate::desktop::Apartment::new().unwrap();
        let mut app = pins();
        app.tray_recovery.set(true);
        app.documents[1].state.shake.pending.set(true);
        app.settle_pin_shake();
        assert_eq!(app.pin_layout.signals.count.get(), 2);
        assert!(!IsWindowVisible(app.documents[2].window.0).as_bool());
        app.documents[2].state.group_hidden.set(true);
        DestroyWindow(app.documents[1].window.0).unwrap();
        app.settle_pin_shake();
        assert_eq!(app.pin_layout.signals.count.get(), 0);
        assert!(IsWindowVisible(app.documents[3].window.0).as_bool());
        assert!(!IsWindowVisible(app.documents[2].window.0).as_bool());
        app.shutdown();
    })
}
#[test]
fn pin_context_menu_exposes_all_visible_pins_layout_and_global_pass_through_toggle() {
    crate::desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        let _apt = crate::desktop::Apartment::new().unwrap();
        let mut app = pins();
        let id = app.documents[1].state.unit_id.clone();
        let snapshot = menu::Snapshot::capture(&app, &id);
        let menu_handle = crate::desktop::context_menu::Menu(CreatePopupMenu().unwrap());
        snapshot.append(menu_handle.0).unwrap();
        let tree = crate::native_menu::snapshot(menu_handle.0).unwrap();
        assert!(tree
            .iter()
            .any(|item| item.label == crate::ui_text::text("workspace-layout-open")));
        assert!(tree
            .iter()
            .any(|item| item.label == crate::ui_text::text("workspace-shake-hide-others")));
        assert!(!tree
            .iter()
            .any(|item| item.label == crate::ui_text::text("pin-pass-enter")));
        // Toggle hide-others and show-all directly from context menu.
        assert!(snapshot.execute(&mut app, &id, menu::SHAKE_TOGGLE).unwrap());
        assert_eq!(app.pin_layout.signals.count.get(), 2);
        let hidden_snap = menu::Snapshot::capture(&app, &id);
        let hidden_menu = crate::desktop::context_menu::Menu(CreatePopupMenu().unwrap());
        hidden_snap.append(hidden_menu.0).unwrap();
        let hidden_tree = crate::native_menu::snapshot(hidden_menu.0).unwrap();
        assert!(hidden_tree
            .iter()
            .any(|item| item.label == crate::ui_text::text("workspace-shake-show-all")));
        assert!(hidden_snap
            .execute(&mut app, &id, menu::SHAKE_TOGGLE)
            .unwrap());
        assert_eq!(app.pin_layout.signals.count.get(), 0);
        // Arrange all visible pins directly via command and custom gap.
        let fresh_snap = menu::Snapshot::capture(&app, &id);
        app.pin_layout.gap_dip.set(24);
        assert!(fresh_snap.execute(&mut app, &id, 0x2191).unwrap());
        let target_y = frame(app.documents[1].window.0).unwrap().y;
        assert_eq!(frame(app.documents[2].window.0).unwrap().y, target_y);
        assert_eq!(frame(app.documents[3].window.0).unwrap().y, target_y);
        assert_eq!(
            crate::shortcut_policy::Policy::default().chord("pin.restore-input"),
            Some(crate::shortcut_policy::Chord {
                key: b'4',
                modifiers: crate::shortcut_policy::CTRL,
            })
        );
        // Verify global pass-through toggle signal & tray label reflect workspace count without blocking.
        app.tray_recovery.set(true);
        for d in &mut app.documents {
            d.state.tray.recovery = app.tray_recovery.clone();
        }
        app.pin_layout.click.toggle_all.set(true);
        app.settle_pin_click(false);
        app.shutdown();
    })
}

#[test]
fn pin_arrange_palette_modal_executes_selected_and_all_scopes_while_owner_disabled() {
    crate::desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        let _apt = crate::desktop::Apartment::new().unwrap();
        let mut app = pins();
        app.sync_workspace_chrome(true);
        for idx in [1usize, 2] {
            app.chrome.select_for_test(app.documents[idx].window.0);
        }
        let anchor_id = app.documents[1].state.unit_id.clone();
        let owner_raw = app.documents[1].window.0 .0 as isize;
        let snapshot = menu::Snapshot::capture(&app, &anchor_id);
        assert_eq!(snapshot.selected_count(), 2);
        assert_eq!(snapshot.all_count(), 3);
        let tid = windows::Win32::System::Threading::GetCurrentThreadId();
        let worker = std::thread::spawn(move || {
            unsafe extern "system" fn find(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
                let slot = &mut *(lp.0 as *mut (isize, HWND));
                if hwnd.0 as isize != slot.0 && IsWindowVisible(hwnd).as_bool() {
                    let mut class = [0u16; 16];
                    let len = GetClassNameW(hwnd, &mut class) as usize;
                    if len == 6 && String::from_utf16_lossy(&class[..len]) == "#32770" {
                        slot.1 = hwnd;
                        return false.into();
                    }
                }
                true.into()
            }
            let mut dialog = HWND::default();
            for _ in 0..300 {
                let mut state = (owner_raw, HWND::default());
                let _ = EnumThreadWindows(tid, Some(find), LPARAM(&mut state as *mut _ as isize));
                if !state.1.is_invalid() {
                    dialog = state.1;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(!dialog.is_invalid(), "pin arrange palette did not open");
            // Default scope is Selected (2 pins): align Top (400 + 3 = 403).
            let _ = SendMessageW(dialog, WM_COMMAND, Some(WPARAM(403)), None);
            // Switch to All (301), set gap to 20, and run CompactX (400 + 8 = 408).
            let _ = SendMessageW(dialog, WM_COMMAND, Some(WPARAM(301)), None);
            let _ = SetDlgItemTextW(dialog, 502, &HSTRING::from("20"));
            let _ = SendMessageW(dialog, WM_COMMAND, Some(WPARAM(408)), None);
            let _ = PostMessageW(
                Some(dialog),
                WM_COMMAND,
                WPARAM(IDCANCEL.0 as usize),
                LPARAM(0),
            );
        });
        assert!(snapshot
            .execute(&mut app, &anchor_id, menu::OPEN_PALETTE)
            .unwrap());
        worker.join().unwrap();
        let r1 = frame(app.documents[1].window.0).unwrap();
        let r2 = frame(app.documents[2].window.0).unwrap();
        let r3 = frame(app.documents[3].window.0).unwrap();
        assert_eq!(r1.y, r2.y, "Selected Top alignment should align doc 1 & 2");
        assert!(
            r2.x == r1.x + r1.width as i32 + 20 && r3.x == r2.x + r2.width as i32 + 20,
            "All CompactX with 20 DIP gap failed: r1={r1:?}, r2={r2:?}, r3={r3:?}"
        );
        app.undo_layout().unwrap();
        app.shutdown();
    })
}
