//! 工作区 chrome 的独立 UI owner；回调只持 Rc 数据，不持 Application 或文档裸指针。
use super::*;
use crate::desktop::{self, link_catalog::Catalog};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::{Rc, Weak},
};
#[path = "workspace_chrome_capsule.rs"]
mod capsule;
#[path = "workspace_chrome_draw.rs"]
mod draw;
#[path = "workspace_chrome_input.rs"]
mod input;
#[path = "workspace_chrome_application.rs"]
mod integration;
#[path = "workspace_chrome_keyboard.rs"]
mod keyboard;
#[path = "workspace_chrome_model.rs"]
mod model;
#[path = "workspace_port_settings.rs"]
pub(in crate::desktop) mod ports;
#[path = "workspace_chrome_scene.rs"]
mod scene;
#[cfg(test)]
#[path = "workspace_chrome_tests.rs"]
pub(in crate::desktop) mod tests;
#[path = "workspace_chrome_window.rs"]
mod window;
use model::{Edge, Mode, Node, Point, Port, Rect, Side};
pub(in crate::desktop) type Binding = RefCell<Weak<Ui>>;
pub(in crate::desktop) struct Ui {
    scene: RefCell<Option<Rc<scene::Scene>>>,
    nodes: RefCell<Vec<Node>>,
    mode: Cell<Mode>,
    disclosed: RefCell<HashSet<Port>>,
    press: RefCell<Option<input::Press>>,
    pending: RefCell<Option<desktop::link_catalog::Creation>>,
    port_settings: RefCell<Option<ports::Open>>,
    keyboard: keyboard::Control,
    capsule: RefCell<Option<Rc<capsule::Open>>>,
    drawing: Cell<HWND>,
    input_windows: RefCell<Vec<(HWND, usize, Side)>>,
    gates: RefCell<Option<scene::Gates>>,
    closed: Cell<bool>,
    failure: RefCell<Option<Error>>,
    reported: Cell<Option<windows::core::HRESULT>>,
}
impl Default for Ui {
    fn default() -> Self {
        Self {
            scene: RefCell::default(),
            nodes: RefCell::default(),
            mode: Cell::default(),
            disclosed: RefCell::default(),
            press: RefCell::default(),
            pending: RefCell::default(),
            port_settings: RefCell::default(),
            keyboard: keyboard::Control::default(),
            capsule: RefCell::default(),
            drawing: Cell::default(),
            input_windows: RefCell::default(),
            gates: RefCell::default(),
            closed: Cell::new(false),
            failure: RefCell::default(),
            reported: Cell::default(),
        }
    }
}
impl Ui {
    fn fail(&self, error: Error) {
        if self.reported.get() != Some(error.code()) && self.failure.borrow().is_none() {
            eprintln!("native-workspace-chrome: {error}");
            *self.failure.borrow_mut() = Some(error);
        }
    }
    fn blocked(&self) -> bool {
        self.closed.get()
            || self
                .gates
                .borrow()
                .as_ref()
                .is_none_or(scene::Gates::blocked)
    }
    fn idle(&self) -> bool {
        self.press.borrow().is_none()
            && self.pending.borrow().is_none()
            && self.port_settings.borrow().is_none()
            && self.keyboard.source().is_none()
            && capsule::idle(self)
    }
    fn node(&self, endpoint: usize) -> Option<Node> {
        self.nodes
            .borrow()
            .iter()
            .find(|node| node.endpoint == endpoint)
            .cloned()
    }
    fn point(&self, port: Port) -> Option<Point> {
        self.node(port.node)?.point(port.side, port.index)
    }
    fn visible(&self, port: Port) -> bool {
        self.point(port).is_some()
    }
    unsafe fn cancel(&self) {
        self.keyboard.cancel_link();
        self.press.borrow_mut().take();
        let captured = windows::Win32::UI::Input::KeyboardAndMouse::GetCapture();
        let ours = self
            .input_windows
            .borrow()
            .iter()
            .any(|(hwnd, _, _)| *hwnd == captured);
        if !captured.is_invalid() && ours {
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
        }
    }
    unsafe fn repaint(&self) {
        for hwnd in self.handles().into_iter().filter(|h| !h.is_invalid()) {
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
        }
    }
    fn handles(&self) -> Vec<HWND> {
        std::iter::once(self.drawing.get())
            .chain(self.input_windows.borrow().iter().map(|(hwnd, _, _)| *hwnd))
            .collect()
    }
}
#[derive(Default)]
pub(super) struct Owner {
    ui: Rc<Ui>,
    windows: Option<window::Windows>,
}
impl Owner {
    pub(super) fn bind(&self) -> Weak<Ui> {
        Rc::downgrade(&self.ui)
    }
    pub(super) fn idle(&self) -> bool {
        self.ui.idle()
    }
    pub(super) fn handles(&self) -> Vec<HWND> {
        self.ui.handles()
    }
    pub(super) fn take(&self) -> Option<desktop::link_catalog::Creation> {
        if self.ui.blocked() {
            self.ui.pending.borrow_mut().take();
            return None;
        }
        let request = self.ui.pending.borrow_mut().take()?;
        let scene = self.ui.scene.borrow().clone()?;
        if !Rc::ptr_eq(&request.catalog, &scene.catalog) {
            return None;
        }
        for unit in [&request.link.from_unit, &request.link.to_unit] {
            let endpoint = scene
                .catalog
                .endpoints
                .iter()
                .position(|e| &e.unit == unit)?;
            let node = self.ui.node(endpoint)?;
            let hwnd = HWND(node.hwnd as *mut _);
            if !node.interactive
                || !unsafe {
                    IsWindowVisible(hwnd).as_bool()
                        && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == node.marker
                }
            {
                return None;
            }
        }
        Some(request)
    }
    pub(super) unsafe fn close(&mut self) {
        self.ui.closed.set(true);
        capsule::cancel(&self.ui);
        self.ui.keyboard.retire();
        self.ui.cancel();
        self.ui.pending.borrow_mut().take();
        self.ui.port_settings.borrow_mut().take();
        self.windows.take();
        self.ui.scene.borrow_mut().take();
        self.ui.nodes.borrow_mut().clear();
    }
}
/// 独立 Live 测试/宿主未绑定时保留旧边角；应用绑定后严格服从三态。
pub(in crate::desktop) fn live_frame(binding: &Binding) -> bool {
    binding
        .borrow()
        .upgrade()
        .is_none_or(|ui| ui.mode.get() != Mode::None)
}
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            self.close();
        }
    }
}
/// 几何变化可发生在系统拖动的嵌套消息泵内；只更新本窗口的标量，不访问 Application。
pub(in crate::desktop) unsafe fn geometry(binding: &Binding, hwnd: HWND, msg: u32) {
    let Some(ui) = binding.borrow().upgrade() else {
        return;
    };
    if matches!(msg, WM_DESTROY | WM_NCDESTROY) {
        capsule::cancel(&ui);
        ui.nodes
            .borrow_mut()
            .retain(|node| node.hwnd != hwnd.0 as isize);
        ui.cancel();
    } else if matches!(msg, WM_WINDOWPOSCHANGED | WM_DPICHANGED) {
        let rect = scene::frame(hwnd);
        if let Some(node) = ui
            .nodes
            .borrow_mut()
            .iter_mut()
            .find(|node| node.hwnd == hwnd.0 as isize)
        {
            if let Some(rect) = rect {
                node.rect = rect;
                node.dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
            } else {
                node.rect = Rect::default();
            }
        }
    } else {
        return;
    }
    if let Err(error) = window::refresh(&ui) {
        window::hide(&ui);
        ui.fail(error);
    }
}
