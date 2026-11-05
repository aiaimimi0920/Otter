//! 接收保存后的呈现负向与确认身份；所有窗口/配置为当前测试自有资源。
use super::*;

unsafe fn prevented(mode: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    menu_receive(&mut fixture);
    let journal = fixture.journal();
    checkpoint(&mut fixture.app);
    let doc = fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
        .unwrap();
    match mode {
        0 => doc.state.canvas.borrow_mut().window_opacity = 0.0,
        1 => doc.state.group_hidden.set(true),
        2 => {
            let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
            SendMessageW(doc.window.0, WM_PAINT, None, None);
            assert!(doc.state.canvas.borrow().confirmed_paints > 0);
            let _ = ShowWindow(doc.window.0, SW_HIDE);
            doc.state
                .canvas
                .borrow_mut()
                .replace_image(crate::image_decode::Image {
                    width: 1,
                    height: 1,
                    pixels: vec![0, 0, 0, 255].into(),
                })
                .unwrap();
        }
        3 => fixture
            .app
            .authorization_owner
            .invalidate(&fixture.origin)
            .unwrap(),
        _ => unreachable!(),
    }
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
    assert!(!fixture.app.projection_receive.busy());
    assert_eq!(
        fixture.app.authorization.borrow().projection.status,
        "接收未完成；本地内容和未结意图保留"
    );
    assert_eq!(fixture.journal(), journal);
    assert!(fixture.unit_present());
}
#[test]
fn projection_present_zero_opacity_cannot_authorize_displayed() {
    unsafe { prevented(0) }
}
#[test]
fn projection_present_hidden_group_cannot_authorize_displayed() {
    unsafe { prevented(1) }
}
#[test]
fn projection_present_old_frame_cannot_authorize_replaced_image() {
    unsafe { prevented(2) }
}
#[test]
fn projection_present_revoked_original_grant_cannot_show_or_report_saved_image() {
    unsafe { prevented(3) }
}

#[test]
fn projection_accept_confirmation_requires_same_arc_and_live_requester_without_borrowing_view() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let doc = &app.documents[0];
        let origin = "https://loom.example";
        let entry = Arc::new(png::invitation(origin));
        let mut view = doc.state.authorization.borrow_mut();
        view.origin = Some(origin.into());
        view.projection.entries = vec![entry.clone()];
        drop(view);
        ui::accept::confirm_with(doc.window.0, &doc.state, ui::ACCEPT_BASE, |_| false).unwrap();
        assert!(app.authorization.borrow().request.is_none());
        assert!(
            ui::accept::confirm_with(doc.window.0, &doc.state, ui::ACCEPT_BASE, |_| {
                doc.state.authorization.borrow_mut().projection.entries[0] =
                    Arc::new((*entry).clone());
                true
            })
            .is_err()
        );
        assert!(app.authorization.borrow().request.is_none());
        ui::accept::confirm_with(doc.window.0, &doc.state, ui::ACCEPT_BASE, |_| true).unwrap();
        assert!(matches!(
            app.authorization.borrow_mut().request.take(),
            Some(Request::Projection(ui::Request::Accept(_)))
        ));
        ui::accept::confirm_with(doc.window.0, &doc.state, ui::ACCEPT_BASE, |_| true).unwrap();
        app.process_authorization_with_projection(|_| {
            panic!("unsafe destination must fail before config/network admission")
        });
        assert!(!app.remote_sessions.projection_pending());
        assert_eq!(
            app.authorization.borrow().projection.status,
            "接收未启动：当前工作区不能安全保存"
        );
        app.shutdown();
    }
}
#[test]
fn projection_present_zero_dimensions_do_not_confirm_a_frame_or_create_a_render_target() {
    let _com = Apartment::new().unwrap();
    let mut canvas = crate::desktop::Canvas::new(Some(crate::image_decode::Image {
        width: 1,
        height: 1,
        pixels: vec![0, 0, 0, 255].into(),
    }))
    .unwrap();
    for (width, height) in [(0, 1), (1, 0), (0, 0)] {
        canvas.paint(HWND::default(), width, height, 96).unwrap();
        assert_eq!(canvas.confirmed_paints, 0);
        assert_eq!(canvas.paints, 0);
    }
}
