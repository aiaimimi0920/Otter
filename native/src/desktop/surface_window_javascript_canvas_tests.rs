//! 真 child 的 Canvas 像素经生产图片 owner 进入 HWND；不走测试专用绘图器。
use super::{
    css_tests::{geometry, pixel, pixels},
    *,
};
use crate::surface_javascript::{canvas_tests::SOURCE, Frame};
fn install(window: &SurfaceWindow, frame: Frame) {
    let images = frame
        .images
        .into_iter()
        .map(|(id, image)| (id, crate::surface_image_paint::Bitmap::new(image).unwrap()))
        .collect();
    window.replace_with_images(frame.scene, images).unwrap();
}
impl SurfaceWindow {
    pub(crate) fn javascript_canvas_pixel_for_test(&self, x: usize, y: usize) -> [u8; 4] {
        pixel(&pixels(self), x, y).try_into().unwrap()
    }
}
#[test]
fn javascript_canvas_real_hwnd_pixels_nonuniform_fit_geometry_frames_resize_and_close() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"placeholder","type":"column"}"#).unwrap(),
        200,
        80,
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
    let id = frame.scene.root().children()[0].id().to_owned();
    install(&window, frame);
    let width = window.viewport().unwrap().logical()[0] * 0.5;
    assert_eq!(geometry(&window, &id), Some(([0., 0.], [width, 40.])));
    assert_eq!(
        window.javascript_canvas_pixel_for_test(2, 2),
        [0, 255, 0, 255]
    );
    assert_eq!(
        window.javascript_canvas_pixel_for_test(85, 20),
        [0, 255, 0, 255]
    ); // fill 而非 contain 留黑边。
    install(&window, session.animation_frame(&|| false).unwrap());
    assert_eq!(
        window.javascript_canvas_pixel_for_test(2, 2),
        [255, 0, 0, 255]
    );
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            300,
            100,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
    }
    session.set_viewport(window.viewport().unwrap()).unwrap();
    install(&window, session.animation_frame(&|| false).unwrap());
    assert!(
        window.javascript_resize_report_for_test()["count"]
            .as_u64()
            .unwrap()
            >= 2
    );
    install(&window, session.animation_frame(&|| false).unwrap());
    assert_eq!(
        window.javascript_canvas_pixel_for_test(2, 2),
        [0, 255, 0, 255]
    );
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}
