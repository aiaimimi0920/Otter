//! 真 JS child → DOM → DirectWrite/D2D HWND → 原生点击候选 → JS emit → 新快照 → 关闭。
//! 自有隐藏窗口和定向消息，不抢前台/全局输入；不是用户可见产品或真实 Loom 验收。
use super::super::*;
use crate::surface_javascript::{fixture, Session};
#[path = "surface_window_javascript_animation_tests.rs"]
mod animation_tests;
#[path = "surface_window_javascript_border_tests.rs"]
mod border_tests;
#[path = "surface_window_javascript_canvas_tests.rs"]
mod canvas_tests;
#[path = "surface_window_javascript_css_hover_tests.rs"]
mod css_hover_tests;
#[path = "surface_window_javascript_css_tests.rs"]
mod css_tests;
#[path = "surface_window_javascript_event_tests.rs"]
mod event_tests;
#[path = "surface_window_javascript_focus_tests.rs"]
mod focus_tests;
#[path = "surface_window_javascript_geometry_tests.rs"]
mod geometry_tests;
#[path = "surface_window_javascript_grid_tests.rs"]
mod grid_tests;
#[path = "surface_window_javascript_hover_tests.rs"]
mod hover_tests;
#[path = "surface_window_javascript_keyboard_tests.rs"]
mod keyboard_tests;
#[path = "surface_window_javascript_nth_tests.rs"]
mod nth_tests;
#[path = "surface_window_javascript_padding_tests.rs"]
mod padding_tests;
#[path = "surface_window_javascript_pointer_tests.rs"]
mod pointer_tests;
#[path = "surface_window_javascript_resize_tests.rs"]
mod resize_tests;
#[path = "surface_window_javascript_spatial_tests.rs"]
mod spatial_tests;
#[path = "surface_window_javascript_text_overflow_tests.rs"]
mod text_overflow_tests;
#[path = "surface_window_javascript_text_tests.rs"]
mod text_tests;
#[path = "surface_window_javascript_timer_tests.rs"]
mod timer_tests;
#[path = "surface_window_javascript_typography_tests.rs"]
mod typography_tests;
#[test]
fn javascript_dom_projects_to_real_window_and_native_click_returns_to_same_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = super::super::super::Apartment::new().unwrap();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        fixture::SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let button = frame.scene.root().children()[1].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    let target = window.javascript_paint_click_for_test(&button, &binding, 3);
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(frame.events.len(), 1);
    let event: serde_json::Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(event["payload"]["count"], 1);
    assert_eq!(event["baseRevision"], 3);
    window.replace(frame.scene).unwrap();
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    let node = window.accessibility_node("dom:2").unwrap();
    assert!(node.read().is_some());
    {
        let host = window.state.host.borrow();
        assert_eq!(
            fixture::text(host.as_ref().unwrap().presentation().scene()),
            "原生🙂:45:1"
        );
    }
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    assert!(node.read().is_none());
}
impl SurfaceWindow {
    pub(crate) fn javascript_focus_request_for_test(&self, focus: bool) {
        let id = self
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presentation()
            .scene()
            .root()
            .children()[if focus { 2 } else { 3 }]
        .id()
        .to_owned();
        self.javascript_queue_click_for_test(&id);
    }
    pub(crate) fn javascript_first_button_for_test(&self) -> String {
        fn find(node: &crate::surface_scene::Node) -> Option<&str> {
            if node.kind() == crate::surface_scene::Kind::Button {
                Some(node.id())
            } else {
                node.children().iter().find_map(find)
            }
        }
        let host = self.state.host.borrow();
        find(host.as_ref().unwrap().presentation().scene().root())
            .expect("fixture button missing")
            .to_owned()
    }
    pub(crate) fn javascript_text_for_test(&self) -> String {
        let host = self.state.host.borrow();
        fixture::text(host.as_ref().unwrap().presentation().scene()).to_owned()
    }
    pub(crate) fn javascript_apartment_for_test() -> impl Drop {
        super::super::super::Apartment::new().unwrap()
    }
    // 只用于本地/授权 HTTPS 两个真 child 夹具；不扩大产品 HWND/点击 API 的可见性。
    pub(crate) fn javascript_paint_click_for_test(
        &self,
        button: &str,
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
    ) -> String {
        self.set_pointer_enabled(true).unwrap();
        assert_eq!(self.javascript_text_for_test(), "原生🙂:42:0");
        self.javascript_queue_click_for_test(button);
        let candidate = self
            .take_click()
            .expect("real HWND pointer did not produce a candidate");
        let target = candidate.node_id.clone();
        assert!(self
            .click_events_for_candidate(candidate, binding, revision)
            .unwrap()
            .is_empty());
        target
    }
    /// 不开权限、不消费候选；Application 必须自己完成 sync_input/listener/batch 排程。
    pub(crate) fn javascript_queue_click_for_test(&self, button: &str) {
        assert!(self.pointer_enabled());
        unsafe {
            SendMessageW(self.hwnd, WM_PAINT, None, None);
        }
        assert!(self.take_error().is_none());
        let point = {
            let host = self.state.host.borrow();
            let presentation = host.as_ref().unwrap().presentation();
            let geometry = presentation
                .geometry()
                .nodes()
                .iter()
                .find(|node| node.id() == button)
                .unwrap();
            let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(self.hwnd) } as f32 / 96.;
            std::array::from_fn::<_, 2, _>(|axis| {
                ((geometry.origin()[axis] + geometry.size()[axis] / 2.) * dpi).round() as i32
            })
        };
        assert_eq!(self.hit_test(point).unwrap().as_deref(), Some(button));
        let packed = (point[0] as u16 as u32) | ((point[1] as u16 as u32) << 16);
        unsafe {
            SendMessageW(
                self.hwnd,
                WM_LBUTTONDOWN,
                None,
                Some(LPARAM(packed as isize)),
            );
            SendMessageW(self.hwnd, WM_LBUTTONUP, None, Some(LPARAM(packed as isize)));
        }
        assert!(self.has_pending_click());
    }
}
