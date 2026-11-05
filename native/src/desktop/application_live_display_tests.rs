//! 隐藏 Live/WARP 首次快照进入真实显示图；原图不覆盖，断线/关窗/替换使缓存退休。
use super::*;
fn settle_display(app: &Application) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.sync_display_frames();
        if app.documents[0].state.resolved_display.borrow().is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "Live display timeout");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn drain_display(app: &Application) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.sync_display_frames();
        if !app.display_composition.borrow().is_busy() {
            break;
        }
        assert!(Instant::now() < deadline, "Live display cleanup timeout");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn live_display_frame_relays_reuses_pixels_and_disconnect_drops_snapshot() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (_, unit) = live(&mut app, true);
        super::execution::connect(&mut app, &unit);
        app.add_window(false).unwrap();
        let target = app.documents[0].state.unit_id.clone();
        let relay = app.documents[1].state.unit_id.clone();
        app.workspace
            .connect(crate::workspace::Link {
                id: crate::workspace_id::UnitId::parse("live-relay").unwrap(),
                from_unit: target,
                from_port: "output".into(),
                to_unit: relay,
                to_port: "image".into(),
            })
            .unwrap();
        settle_display(&app);
        let (identity, pixels) = {
            let state = &app.documents[0].state;
            let frame = state.resolved_display.borrow();
            let frame = frame.as_ref().unwrap();
            assert!(frame.is_from(&unit));
            assert_eq!((frame.image.width, frame.image.height), (2, 2));
            assert_eq!(&*frame.image.pixels, &[0, 0, 255, 255].repeat(4));
            assert!(state.canvas.borrow().image.is_none());
            (frame.identity.clone(), frame.image.pixels.clone())
        };
        assert!(app.documents[1]
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .image
            .pixels
            .same_storage(&pixels));
        // 释放借用后反复处理普通消息，不重新读回、不替换显示身份。
        for _ in 0..32 {
            app.sync_display_frames();
        }
        assert!(!app.display_composition.borrow().is_busy());
        assert!(Arc::ptr_eq(
            &identity,
            &app.documents[0]
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .identity
        ));
        let edge = app.workspace.links()[0].id.clone();
        app.workspace.remove_link(&edge);
        app.sync_display_frames();
        assert!(app
            .documents
            .iter()
            .all(|doc| doc.state.resolved_display.borrow().is_none()));
        app.shutdown_live().unwrap();
    }
}

#[test]
fn live_display_closed_pending_and_replaced_workspace_never_publish_old_frame() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..2 {
            let mut app = environment.app();
            let (hwnd, unit) = live(&mut app, true);
            super::execution::connect(&mut app, &unit);
            app.sync_display_frames();
            assert!(app.display_composition.borrow().is_busy());
            if mode == 0 {
                DestroyWindow(hwnd).unwrap();
            } else {
                settle_display(&app);
                let links = app.workspace.links().to_vec();
                app.workspace = crate::workspace::Workspace::with_initial(
                    app.documents[0].state.unit_id.clone(),
                );
                app.workspace.add_units([unit.clone()]).unwrap();
                app.workspace.connect(links[0].clone()).unwrap();
            }
            drain_display(&app);
            assert!(app.documents[0].state.resolved_display.borrow().is_none());
            app.shutdown_live().unwrap();
        }
    }
}

#[test]
fn live_display_art_overlay_belongs_to_target_and_empty_live_has_no_image() {
    unsafe {
        let environment = Environment::new();
        for with_frame in [false, true] {
            let mut app = environment.app();
            let (_listener, _) = super::execution::select(&app);
            let (_, unit) = live(&mut app, with_frame);
            super::execution::connect(&mut app, &unit);
            if with_frame {
                settle_display(&app);
                let doc = &app.documents[0];
                let frame = doc.state.resolved_display.borrow();
                let frame = frame.as_ref().unwrap();
                assert!(frame.is_from(&unit));
                assert!(frame.has_overlay_owner(&doc.state.unit_id));
                assert!(frame.editor.is_some());
                assert!(doc.state.canvas.borrow().image.is_none());
            } else {
                for _ in 0..8 {
                    app.sync_display_frames();
                }
                assert!(!app.display_composition.borrow().is_busy());
                assert!(app.documents[0].state.resolved_display.borrow().is_none());
            }
            app.shutdown_live().unwrap();
        }
    }
}
