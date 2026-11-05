//! 真实隐藏 HWND 图片提交、上传和回收；不冒充远端资源授权或桌面像素截图验收。
use super::*;
use crate::{image_decode::Image, surface_image_paint::Bitmap, surface_presentation::Images};
#[test]
fn surface_image_unresolved_projection_omits_layout_hit_and_accessibility_but_keeps_siblings() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"gap":7},"children":[{"id":"pending","type":"image","layout":{"height":100}},{"id":"button","type":"button","layout":{"height":30},"props":{"label":"Available"}},{"id":"text","type":"text","props":{"text":"Still visible"}}]}"#).unwrap().with_image_availability(|_| false);
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    paint(&window);
    let host = window.state.host.borrow();
    let geometry = host.as_ref().unwrap().presentation().geometry();
    assert_eq!(geometry.nodes().len(), 3);
    assert_eq!(geometry.nodes()[1].origin()[1], 0.);
    assert_eq!(geometry.nodes()[2].origin()[1], 37.);
    drop(host);
    assert!(window.accessibility_node("pending").is_none());
    assert!(window.accessibility_node("button").is_some());
    // 避开按钮圆角外的空白区域；此处检查重新排布后的按钮内部命中。
    assert_eq!(
        window.hit_test([20, 15]).unwrap().as_deref(),
        Some("button")
    );
    window.close().unwrap();
}
fn image(width: u32, height: u32) -> Rc<Bitmap> {
    Bitmap::new(Image {
        width,
        height,
        pixels: vec![255; (width * height * 4) as usize].into(),
    })
    .unwrap()
}
fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"image","type":"image"}]}"#)
        .unwrap()
}
fn images(image: Rc<Bitmap>) -> Images {
    [("image".into(), image)].into()
}
fn paint(window: &SurfaceWindow) {
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
}
fn size(window: &SurfaceWindow) -> [f32; 2] {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()[1]
        .size()
}
#[test]
fn surface_image_hwnd_paints_reflows_and_releases_target_on_close() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let source = image(80, 40);
    let weak = Rc::downgrade(&source);
    let window = SurfaceWindow::with_images(scene(), 240, 200, images(source.clone())).unwrap();
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    assert!(!source.uploaded_for_test());
    paint(&window);
    assert!(source.uploaded_for_test());
    let before = size(&window);
    assert_eq!(before[0], before[1] * 2.);
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            320,
            240,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
    }
    window.hit_test([1, 1]).unwrap();
    assert!(
        !source.uploaded_for_test(),
        "viewport change must release the old target upload"
    );
    assert!(size(&window)[0] > before[0]);
    paint(&window);
    assert!(source.uploaded_for_test());
    window.close().unwrap();
    assert!(!source.uploaded_for_test());
    drop(source);
    assert!(
        weak.upgrade().is_none(),
        "closed HWND must not retain CPU pixels"
    );
}
#[test]
fn surface_image_fractional_hwnd_uses_logical_ratio_and_releases_upload() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let source = Bitmap::with_logical_size(
        Image {
            width: 3,
            height: 2,
            pixels: vec![255; 24].into(),
        },
        [2.5, 1.5],
    )
    .unwrap();
    let window = SurfaceWindow::with_images(scene(), 240, 200, images(source.clone())).unwrap();
    paint(&window);
    let initial = size(&window);
    assert!((initial[1] - initial[0] * 0.6).abs() < 0.001);
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            320,
            240,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
    }
    window.hit_test([1, 1]).unwrap();
    let resized = size(&window);
    assert!(resized[0] > initial[0]);
    assert!((resized[1] - resized[0] * 0.6).abs() < 0.001);
    paint(&window);
    assert!(source.uploaded_for_test());
    window.close().unwrap();
    assert!(!source.uploaded_for_test());
}
#[test]
fn surface_image_hwnd_replace_requires_new_resources_and_releases_old_upload() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let first = image(80, 40);
    let window = SurfaceWindow::with_images(scene(), 240, 600, images(first.clone())).unwrap();
    paint(&window);
    let before = size(&window);
    assert!(window.replace(scene()).is_err());
    assert_eq!(size(&window), before);
    assert!(first.uploaded_for_test());
    // 失败保持旧呈现，但遵守既有窗口规则撤销描述，不能把旧 UIA 身份视为新提交。
    assert!(window.accessibility_node("image").is_none());
    let second = image(40, 80);
    window
        .replace_with_images(scene(), images(second.clone()))
        .unwrap();
    assert!(!first.uploaded_for_test());
    assert_eq!(size(&window)[1], size(&window)[0] * 2.);
    paint(&window);
    assert!(second.uploaded_for_test());
    window.replace(super::scene("empty")).unwrap();
    assert!(!second.uploaded_for_test());
    window.close().unwrap();
}
#[test]
fn surface_image_hwnd_rejects_missing_busy_and_closed_candidates_without_retaining_pixels() {
    let _com = crate::desktop::Apartment::new().unwrap();
    assert!(SurfaceWindow::new(scene(), 240, 200).is_err());
    let window = SurfaceWindow::new(super::scene("empty"), 240, 200).unwrap();
    let held = window.state.host.borrow_mut().take();
    let candidate = image(1, 1);
    let weak = Rc::downgrade(&candidate);
    assert!(window
        .replace_with_images(scene(), images(candidate))
        .is_err());
    assert!(weak.upgrade().is_none());
    *window.state.host.borrow_mut() = held;
    window.close().unwrap();
    let candidate = image(1, 1);
    let weak = Rc::downgrade(&candidate);
    assert!(window
        .replace_with_images(scene(), images(candidate))
        .is_err());
    assert!(weak.upgrade().is_none());
}
