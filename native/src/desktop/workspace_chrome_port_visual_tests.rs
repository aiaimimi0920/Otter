//! 真实端口 HWND 的绘制、region、输入和连线锚点共享主题胶囊；不注入全局输入。
use super::*;
#[test]
fn expanded_port_hit_region_and_link_endpoint_follow_the_outer_anchor_without_new_windows() {
    unsafe {
        let mut app = visible_app();
        let ui = &app.chrome.ui;
        let handles = ui.handles();
        let (left, left_port, left_point) = port(ui, 0, Side::Input);
        let (right, right_port, right_point) = port(ui, 0, Side::Output);
        assert_eq!(input::hit(ui, left, left_point), Some(left_port));
        assert_eq!(input::hit(ui, right, right_point), Some(right_port));
        assert_eq!(input::hit(ui, left, right_point), None);
        assert_eq!(input::hit(ui, right, left_point), None);
        for side in [Side::Input, Side::Output] {
            let (hwnd, port, _) = port(ui, 0, side);
            let node = ui.node(0).unwrap();
            let edge = ui.point(port).unwrap();
            assert_eq!(edge.y, node.rect.top + 36 * node.dpi as i32 / 96);
            ui.disclosed.borrow_mut().insert(port, true);
            app.chrome.windows.as_ref().unwrap().refresh().unwrap();
            UpdateWindow(hwnd).ok().unwrap();
            let expanded = ui.point(port).unwrap();
            let layout = port_visual::layout(ui, port).unwrap();
            let width = 256 * node.dpi as i32 / 96;
            let delta = width - 28 * node.dpi as i32 / 96;
            assert_eq!(
                expanded.x,
                edge.x + if side == Side::Input { -delta } else { delta }
            );
            assert_eq!(expanded.y, edge.y);
            let geometry = port_visual::SideGeometry::new(ui, &node, side);
            let region = geometry.rect(port.index, &layout);
            assert_eq!(region.right - region.left, width);
            assert_eq!((expanded.x, expanded.y), layout.connection());
            let (_, _, visible) = self::port(ui, 0, side);
            assert!(occlusion::contains(hwnd, visible));
            mouse(ui, hwnd, WM_LBUTTONDOWN, visible);
            mouse(ui, hwnd, WM_LBUTTONUP, visible);
            assert_eq!(ui.disclosed.borrow().get(&port), Some(&false));
            assert_eq!(ui.point(port), Some(edge));
        }
        assert_eq!(ui.handles(), handles);
        app.shutdown();
    }
}
