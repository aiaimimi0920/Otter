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
