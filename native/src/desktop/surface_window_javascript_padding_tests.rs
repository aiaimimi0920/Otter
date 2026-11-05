//! 四边声明从真实 child 进入隐藏 HWND 的像素、文字原点与命中；不注入桌面输入。
use super::*;
#[test]
fn javascript_padding_real_hwnd_pixels_text_hits_and_same_child_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../../tests/fixtures/javascript-padding.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let button = frame.scene.root().children()[0].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 200, 80).unwrap();
    let check = |updated| {
        let origin = if updated { [19., 9.] } else { [15., 5.] };
        assert_eq!(
            css_tests::geometry(&window, &button),
            Some((origin, [32., 40.]))
        );
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        let text = &presentation.text().items()[0];
        assert_eq!(text.origin()[0], if updated { 28. } else { 22. });
        assert_eq!(
            text.allocated(),
            if updated { [20., 34.] } else { [22., 34.] }
        );
        assert_eq!(
            presentation.hit_test([origin[0] + 1., origin[1] + 8.]),
            Some(button.as_str())
        );
        assert_eq!(
            presentation.hit_test([origin[0] - 1., origin[1] + 8.]),
            Some(presentation.scene().root().id())
        );
    };
    check(false);
    let before = css_tests::pixels(&window);
    assert_eq!(css_tests::pixel(&before, 1, 1), [255, 0, 0, 255]);
    // 避开原按钮默认圆角，分别采样 padding 区与按钮直边；不以角外点冒充命中。
    assert_eq!(css_tests::pixel(&before, 14, 13), [16, 16, 16, 255]);
    assert_eq!(css_tests::pixel(&before, 16, 13), [0, 255, 0, 255]);
    assert_eq!(css_tests::pixel(&before, 52, 6), [255, 255, 0, 255]);
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    check(true);
    let after = css_tests::pixels(&window);
    assert_eq!(css_tests::pixel(&after, 16, 13), [16, 16, 16, 255]);
    assert_eq!(css_tests::pixel(&after, 20, 18), [0, 255, 0, 255]);
    assert_ne!(before, after);
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}
