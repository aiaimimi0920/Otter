//! 真 timer child 的 DOM/几何/像素沿生产 Presentation 进入 HWND，不手动改图。
use super::{
    css_tests::{geometry, pixel, pixels},
    *,
};
use crate::surface_javascript::timer_tests::{report, SOURCE};
#[test]
fn javascript_timer_real_hwnd_pixels_geometry_resize_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"placeholder","type":"column"}"#).unwrap(),
        400,
        240,
    )
    .unwrap();
    let (mut session, initial) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(window.viewport().unwrap()),
        &|| false,
    )
    .unwrap();
    window.replace(initial.scene).unwrap();
    for round in 1..=2 {
        if round == 2 {
            unsafe {
                SetWindowPos(
                    window.hwnd,
                    None,
                    0,
                    0,
                    600,
                    280,
                    SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
                )
                .unwrap();
            }
        }
        let view = window.viewport().unwrap();
        session.set_viewport(view).unwrap();
        let frame = session.timer_tick(&|| false).unwrap();
        assert_eq!(frame.viewport, Some(view));
        let value = report(&frame);
        let id = frame.scene.root().children()[0].id().to_owned();
        window.replace(frame.scene).unwrap();
        let (_, size) = geometry(&window, &id).unwrap();
        assert!((value["width"].as_f64().unwrap() - f64::from(size[0])).abs() < 0.01);
        assert!((size[0] - view.logical()[0] * round as f32 * 0.1).abs() < 0.01);
        assert_eq!(
            pixel(&pixels(&window), 5, 5),
            if round == 1 {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            }
        );
    }
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}
