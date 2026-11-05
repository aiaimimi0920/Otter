//! 隐藏 HWND 消费同 child 四边框；实际 WndProc/同帧 WIC 像素、内容几何和恢复。
use super::*;
use crate::surface_javascript::border_tests::SOURCE;
#[test]
fn javascript_border_sides_real_hwnd_pixels_geometry_update_and_restore() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    // new 接收窗口外框像素，不是客户区；先创建足够大的 HWND，再绑定其真实视口。
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"placeholder","type":"column"}"#).unwrap(),
        420,
        260,
    )
    .unwrap();
    let (mut session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(window.viewport().unwrap()),
        &|| false,
    )
    .unwrap();
    let node = &frame.scene.root().children()[0];
    let id = node.id().to_owned();
    let text = node.children()[0].id().to_owned();
    assert_eq!(frame.viewport, Some(window.viewport().unwrap()));
    window.replace(frame.scene).unwrap();
    let check = |left, top| {
        assert_eq!(
            css_tests::geometry(&window, &id),
            Some(([0., 0.], [100., 64.]))
        );
        assert_eq!(css_tests::geometry(&window, &text).unwrap().0, [left, top]);
        unsafe {
            SendMessageW(window.hwnd, WM_PAINT, None, None);
        }
        assert!(window.take_error().is_none());
        css_tests::pixels(&window)
    };
    let first = check(11., 5.);
    assert_eq!(css_tests::pixel(&first, 1, 30), [255, 0, 0, 255]);
    assert_eq!(css_tests::pixel(&first, 50, 0), [0, 0, 255, 255]);
    assert_eq!(css_tests::pixel(&first, 50, 62), [0, 128, 0, 255]);
    assert_eq!(css_tests::pixel(&first, 99, 30), [0, 0, 255, 255]);
    {
        let host = window.state.host.borrow();
        assert_eq!(
            host.as_ref().unwrap().presentation().hit_test([1., 30.]),
            Some(id.as_str())
        );
    }
    for (revision, origin) in [(4, [7., 5.]), (5, [3., 3.]), (6, [11., 5.])] {
        let frame = session
            .update(
                fixture::snapshot(revision, 45),
                fixture::resources(),
                &|| false,
            )
            .unwrap();
        window.replace(frame.scene).unwrap();
        let pixels = check(origin[0], origin[1]);
        if revision == 6 {
            assert_eq!(pixels, first);
        } else {
            assert_ne!(pixels, first);
        }
        if revision == 4 {
            assert_eq!(css_tests::pixel(&pixels, 6, 30), [0, 0, 0, 255]);
        }
        if revision == 5 {
            assert_eq!(css_tests::pixel(&pixels, 1, 30), [0, 0, 0, 255]);
        }
    }
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}
