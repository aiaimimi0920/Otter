//! 观察者和 rAF 读取同一 HWND 布局；验证内容盒、最终像素、代次与同 child 清理。
use super::{
    css_tests::{geometry, pixel, pixels},
    *,
};
use crate::surface_javascript::resize_tests::{report, SOURCE};
impl SurfaceWindow {
    pub(crate) fn javascript_resize_report_for_test(&self) -> serde_json::Value {
        let host = self.state.host.borrow();
        serde_json::from_str(
            host.as_ref()
                .unwrap()
                .presentation()
                .scene()
                .root()
                .children()
                .last()
                .unwrap()
                .props()["text"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
    }
}
#[test]
fn javascript_resize_real_hwnd_observer_frame_native_geometry_pixels_and_epoch() {
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
    assert!(initial.resize_observed && initial.animation_pending);
    window.replace(initial.scene).unwrap();
    let first = session.animation_frame(&|| false).unwrap();
    window.replace(first.scene).unwrap();
    assert_eq!(pixel(&pixels(&window), 10, 10), [0, 255, 0, 255]);
    let epoch = window.viewport_epoch();
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
    assert!(window.viewport_epoch() > epoch);
    let view = window.viewport().unwrap();
    session.set_viewport(view).unwrap();
    let observed = session.animation_frame(&|| false).unwrap();
    assert_eq!(report(&observed)["count"], 2);
    assert_eq!(report(&observed)["frames"], 1);
    assert!(observed.animation_pending);
    window.replace(observed.scene).unwrap();
    let drawn = session.animation_frame(&|| false).unwrap();
    let value = report(&drawn);
    let id = drawn.scene.root().children()[0].id().to_owned();
    window.replace(drawn.scene).unwrap();
    let (_, size) = geometry(&window, &id).unwrap();
    assert!((value["bounds"].as_f64().unwrap() - f64::from(size[0])).abs() < 0.01);
    assert!((value["entry"]["content"].as_f64().unwrap() - f64::from(size[0] - 12.)).abs() < 0.01);
    assert_eq!(pixel(&pixels(&window), 10, 10), [255, 0, 0, 255]);
    assert!(!drawn.animation_pending);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}
