//! 同一 child/CSS→真实 HWND/Presentation，裁剪/像素/同步 bounds/命中使用相同布局。
use super::{
    css_tests::{geometry, pixel, pixels},
    *,
};
use crate::surface_javascript::spatial_tests::report;
fn verify(window: &SurfaceWindow, frame: crate::surface_javascript::Frame) -> String {
    assert_eq!(frame.viewport, Some(window.viewport().unwrap()));
    let value = report(&frame.scene);
    let stage = frame.scene.root().children()[0].id().to_owned();
    let children = frame.scene.root().children()[0].children();
    let ids: Vec<_> = children.iter().map(|node| node.id().to_owned()).collect();
    window.replace(frame.scene).unwrap();
    for (id, name) in [
        (stage.as_str(), "stage"),
        (&ids[0], "flow"),
        (&ids[1], "fill"),
        (&ids[2], "overlay"),
    ] {
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
    ids[2].clone()
}
fn hit(window: &SurfaceWindow, point: [f32; 2]) -> Option<String> {
    let host = window.state.host.borrow();
    host.as_ref()
        .unwrap()
        .presentation()
        .hit_test(point)
        .map(str::to_owned)
}
#[test]
fn javascript_spatial_real_hwnd_anchors_clip_pixels_hits_click_and_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"placeholder","type":"column"}"#).unwrap(),
        400,
        240,
    )
    .unwrap();
    let (mut session, frame) = Session::mount_in(
        fixture::binding(),
        include_str!("../../tests/fixtures/javascript-spatial.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(window.viewport().unwrap()),
        &|| false,
    )
    .unwrap();
    let target = verify(&window, frame);
    let buffer = pixels(&window);
    assert_eq!(pixel(&buffer, 1, 1), [0, 255, 0, 255]);
    assert_eq!(pixel(&buffer, 10, 10), [255, 0, 0, 255]); // inset:0 填充 padding box，覆盖正常流白块。
    assert_eq!(pixel(&buffer, 80, 20), [0, 0, 255, 255]);
    assert_eq!(pixel(&buffer, 105, 20), [16, 16, 16, 255]);
    assert_eq!(hit(&window, [80., 20.]).as_deref(), Some(target.as_str()));
    assert_ne!(hit(&window, [105., 20.]).as_deref(), Some(target.as_str()));
    let scale = window.viewport().unwrap().ratio();
    assert_eq!(
        window
            .hit_test([(80. * scale) as i32, (20. * scale) as i32])
            .unwrap()
            .as_deref(),
        Some(target.as_str())
    );
    window.set_pointer_enabled(true).unwrap();
    window.javascript_queue_click_for_test(&target);
    assert_eq!(window.take_click().unwrap().node_id, target);
    let frame = session.click(&target, &|| false).unwrap();
    verify(&window, frame);
    assert_eq!(pixel(&pixels(&window), 105, 20), [0, 255, 255, 255]);
    assert_eq!(hit(&window, [105., 20.]).as_deref(), Some(target.as_str()));
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    verify(&window, frame);
    assert_eq!(geometry(&window, &target), Some(([83., 23.], [50., 30.])));
    assert_eq!(pixel(&pixels(&window), 100, 30), [255, 255, 0, 255]);
    assert_eq!(hit(&window, [100., 30.]).as_deref(), Some(target.as_str()));
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-javascript-spatial-hwnd: actual-client=true anchors=true synchronous-bounds=true padding-clip=true overflow-visible=true pixels=true same-hits=true closed=true");
}
