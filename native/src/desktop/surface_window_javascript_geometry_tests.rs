//! 真 HWND 客户区/DPI 与 child 同步 getter 比较；resize 后显式操作，不伪造 observer。
use super::{
    css_tests::{geometry, pixel, pixels},
    *,
};
use crate::surface_javascript::geometry_tests::report;
fn verify(window: &SurfaceWindow, frame: crate::surface_javascript::Frame) -> String {
    assert_eq!(frame.viewport, Some(window.viewport().unwrap()));
    let value = report(&frame.scene);
    let first = frame.scene.root().children()[0].id().to_owned();
    let second = frame.scene.root().children()[1].id().to_owned();
    window.replace(frame.scene).unwrap();
    for (id, name) in [(&first, "first"), (&second, "second")] {
        let (origin, size) = geometry(window, id).unwrap();
        for (key, number) in [
            ("left", origin[0]),
            ("top", origin[1]),
            ("width", size[0]),
            ("height", size[1]),
        ] {
            assert!(
                (value[name][key].as_f64().unwrap() - f64::from(number)).abs() < 0.01,
                "{name}.{key}"
            );
        }
    }
    assert_eq!(
        value["ratio"].as_f64().unwrap(),
        window.viewport().unwrap().ratio()
    );
    let buffer = pixels(window);
    assert_eq!(pixel(&buffer, 1, 1), [0, 255, 0, 255]);
    assert_eq!(pixel(&buffer, 3, 3), [255, 0, 0, 255]);
    let scale = window.viewport().unwrap().ratio();
    let point = [(10. * scale) as i32, (10. * scale) as i32];
    assert_eq!(
        window.hit_test(point).unwrap().as_deref(),
        Some(first.as_str())
    );
    first
}
#[test]
fn javascript_geometry_real_hwnd_matches_synchronous_mount_and_post_resize_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"placeholder","type":"column"}"#).unwrap(),
        420,
        260,
    )
    .unwrap();
    let (mut session, frame) = Session::mount_in(
        fixture::binding(),
        include_str!("../../tests/fixtures/javascript-geometry.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(window.viewport().unwrap()),
        &|| false,
    )
    .unwrap();
    let first = verify(&window, frame);
    let frame = session.click(&first, &|| false).unwrap();
    verify(&window, frame);
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            520,
            320,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
    }
    session.set_viewport(window.viewport().unwrap()).unwrap();
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    verify(&window, frame);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-javascript-geometry-hwnd: actual-client=true shared-font=true pixels=true hits=true resize-update=true closed=true");
}
