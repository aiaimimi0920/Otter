//! 工作区 chrome 的独立 UI owner；回调只持 Rc 数据，不持 Application 或文档裸指针。
use super::*;
#[path = "workspace_chrome_close.rs"]
mod close;
use crate::desktop::{self, link_catalog::Catalog};
pub(in crate::desktop) use close::hide_pin;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::{Rc, Weak},
};
#[path = "workspace_chrome_backing.rs"]
mod backing;
#[path = "workspace_chrome_border.rs"]
mod border;
#[path = "workspace_candidate_auxiliary.rs"]
mod candidate_auxiliary;
#[path = "workspace_chrome_disclosure.rs"]
mod disclosure;
#[path = "workspace_chrome_draw.rs"]
mod draw;
#[path = "workspace_node_drawer.rs"]
mod drawer;
pub(in crate::desktop) use disclosure::collapse_for_settings;
pub(super) use drawer::tab as drawer_tab;
#[path = "workspace_chrome_input.rs"]
mod input;
#[path = "workspace_chrome_application.rs"]
mod integration;
#[path = "workspace_chrome_keyboard.rs"]
mod keyboard;
#[path = "workspace_chrome_model.rs"]
mod model;
#[path = "workspace_chrome_move.rs"]
mod movement;
#[path = "workspace_parameter_rail.rs"]
mod parameter_rail;
#[path = "workspace_parameter_scroll.rs"]
mod parameter_scroll;
#[path = "workspace_chrome_port_visual.rs"]
mod port_visual;
#[path = "workspace_port_settings.rs"]
pub(in crate::desktop) mod ports;
#[path = "workspace_chrome_scene.rs"]
mod scene;
#[path = "workspace_node_selection.rs"]
mod selection;
pub(in crate::desktop) use selection::observe as select_node;
#[cfg(test)]
#[path = "workspace_chrome_tests.rs"]
pub(in crate::desktop) mod tests;
#[path = "workspace_chrome_window.rs"]
mod window;
use model::{Edge, Mode, Node, Point, Port, Rect, Side};
pub(in crate::desktop) type Binding = RefCell<Weak<Ui>>;
pub(in crate::desktop) use parameter_scroll::{
    close_parameter_rows, focus_parameter, reveal_parameter,
};
pub(in crate::desktop) struct Ui {
    scene: RefCell<Option<Rc<scene::Scene>>>,
    nodes: RefCell<Vec<Node>>,
    mode: Cell<Mode>,
    disclosed: RefCell<std::collections::HashMap<Port, bool>>,
    press: RefCell<Option<input::Press>>,
    pending: RefCell<Option<desktop::link_catalog::Creation>>,
    port_settings: RefCell<Option<ports::Open>>,
    keyboard: keyboard::Control,
    rails: RefCell<Vec<(usize, desktop::document_window::parameter_panel::Inline)>>,
    auxiliary_counts: RefCell<Vec<(usize, usize)>>,
    rail_offsets: RefCell<Vec<(crate::workspace_id::UnitId, f64)>>,
    rail_heights: RefCell<Vec<(isize, u32, i32)>>,
    rail_drag: Cell<Option<parameter_scroll::Drag>>,
    rail_focus: RefCell<Option<parameter_rail::Focus>>,
    rail_layout_dirty: Cell<bool>,
    drawing: Cell<HWND>,
    surface: RefCell<Option<draw::Surface>>,
    port_painter: RefCell<Option<neuro_mini_ui::CapsulePainter>>,
    presenting: Cell<bool>,
    ordering: Cell<bool>,
    input_windows: RefCell<Vec<(HWND, usize, Side)>>,
    drawers: RefCell<Vec<Rc<drawer::Open>>>,
    selected: RefCell<Vec<(isize, isize)>>,
    borders: RefCell<Vec<Rc<border::Border>>>,
    backings: RefCell<Vec<Rc<backing::Backing>>>,
    gates: RefCell<Option<scene::Gates>>,
    closed: Cell<bool>,
    failure: RefCell<Option<Error>>,
    reported: Cell<Option<windows::core::HRESULT>>,
    redraw_after_capture: Cell<bool>,
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
            rails: RefCell::default(),
            auxiliary_counts: RefCell::default(),
            rail_offsets: RefCell::default(),
            rail_heights: RefCell::default(),
            rail_drag: Cell::default(),
            rail_focus: RefCell::default(),
            rail_layout_dirty: Cell::new(false),
            drawing: Cell::default(),
            surface: RefCell::default(),
            port_painter: RefCell::default(),
            presenting: Cell::new(false),
            ordering: Cell::new(false),
            input_windows: RefCell::default(),
            drawers: RefCell::default(),
            selected: RefCell::default(),
            borders: RefCell::default(),
            backings: RefCell::default(),
            gates: RefCell::default(),
            closed: Cell::new(false),
            failure: RefCell::default(),
            reported: Cell::default(),
            redraw_after_capture: Cell::new(false),
        }
    }
}
impl Ui {
    fn capturing(&self) -> bool {
        self.gates
            .borrow()
            .as_ref()
            .is_some_and(|g| g.capture.get())
    }
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
        let rails = self.rails.borrow().clone();
        self.press.borrow().is_none()
            && self.rail_drag.get().is_none()
            && self.pending.borrow().is_none()
            && self.port_settings.borrow().is_none()
            && self.keyboard.source().is_none()
            && rails.iter().all(|(_, panel)| unsafe { panel.idle() })
            && drawer::idle(self)
    }
    fn node(&self, endpoint: usize) -> Option<Node> {
        self.nodes
            .borrow()
            .iter()
            .find(|node| node.endpoint == endpoint)
            .cloned()
    }
    fn point(&self, port: Port) -> Option<Point> {
        let (x, y) = port_visual::layout(self, port)?.connection();
        Some(Point { x, y })
    }
    unsafe fn cancel(&self) {
        self.rail_drag.set(None);
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
            .chain(self.backings.borrow().iter().map(|backing| backing.hwnd))
            .chain(self.input_windows.borrow().iter().map(|(hwnd, _, _)| *hwnd))
            .chain(self.borders.borrow().iter().map(|border| border.hwnd))
            .chain(drawer::handles(self))
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
        self.ui.keyboard.retire();
        self.ui.cancel();
        self.ui.pending.borrow_mut().take();
        self.ui.port_settings.borrow_mut().take();
        self.windows.take();
        self.ui.scene.borrow_mut().take();
        self.ui.nodes.borrow_mut().clear();
        self.ui.drawers.borrow_mut().clear();
        self.ui.selected.borrow_mut().clear();
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
    if msg == WM_WINDOWPOSCHANGED {
        border::order(&ui, hwnd);
        if let Err(error) = window::order_drawing(&ui) {
            ui.fail(error);
        }
    }
    if matches!(msg, WM_DESTROY | WM_NCDESTROY) {
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
                let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
                if node.rect == rect && node.dpi == dpi {
                    return; // 命中 region 变更也发位置通知；不重复提交同一几何或重入宿主绘制。
                }
                node.rect = rect;
                node.dpi = dpi;
            } else {
                node.rect = Rect::default();
            }
        }
    } else {
        return;
    }
    refresh_geometry(&ui, false);
}

pub(in crate::desktop) fn border_visible(binding: &Binding, hwnd: HWND) -> bool {
    binding.borrow().upgrade().is_none_or(|ui| {
        !ui.blocked()
            && ui
                .nodes
                .borrow()
                .iter()
                .any(|node| node.hwnd == hwnd.0 as isize && node.frame(ui.mode.get()))
    })
}
pub(in crate::desktop) unsafe fn composite_frame(binding: &Binding, hwnd: HWND, active: bool) {
    let Some(ui) = binding.borrow().upgrade() else {
        return;
    };
    if let Some(node) = ui
        .nodes
        .borrow_mut()
        .iter_mut()
        .find(|node| node.hwnd == hwnd.0 as isize)
    {
        node.composite_frame = active;
    }
    if !ui.blocked() && !ui.capturing() {
        if let Err(error) = border::refresh(&ui) {
            ui.fail(error);
        }
    }
}

pub(in crate::desktop) unsafe fn prepare_frame(binding: &Binding, hwnd: HWND) -> Result<()> {
    if let Some(ui) = binding.borrow().upgrade() {
        border::prepare(&ui, hwnd)?;
    }
    Ok(())
}

pub(in crate::desktop) fn lease_frame(binding: &Binding, host: HWND) -> Option<HWND> {
    let ui = binding.borrow().upgrade()?;
    if ui.blocked() || ui.capturing() {
        return None;
    }
    border::lease(&ui, host)
}
pub(in crate::desktop) fn release_frame(binding: &Binding, hwnd: HWND) {
    if let Some(ui) = binding.borrow().upgrade() {
        border::release(&ui, hwnd);
    }
}
pub(in crate::desktop) unsafe fn commit_frame(binding: &Binding, host: HWND) -> Result<()> {
    if let Some(ui) = binding.borrow().upgrade() {
        border::commit(&ui, host)?;
    }
    Ok(())
}

/// 目标整帧遮住宿主时同步外框元数据，交接不等待主消息循环再纠正透明度。
pub(in crate::desktop) fn presentation(
    binding: &Binding,
    hwnd: HWND,
    minified: bool,
    opacity: f32,
) {
    if let Some(ui) = binding.borrow().upgrade() {
        if let Some(node) = ui
            .nodes
            .borrow_mut()
            .iter_mut()
            .find(|node| node.hwnd == hwnd.0 as isize)
        {
            node.minified = minified;
            node.border_alpha = desktop::pin_border_style::alpha(opacity);
        }
    }
}

// 缩放的新外框在宿主几何提交之前准备，不能让扩大的图片先遮住旧外框。
pub(in crate::desktop) unsafe fn zoom_frame(
    binding: &Binding,
    hwnd: HWND,
    rect: crate::pin_geometry::PinRect,
) {
    let Some(ui) = binding.borrow().upgrade() else {
        return;
    };
    let translated = if let Some(node) = ui
        .nodes
        .borrow_mut()
        .iter_mut()
        .find(|n| n.hwnd == hwnd.0 as isize)
    {
        let next = Rect {
            left: rect.x,
            top: rect.y,
            right: rect.x.saturating_add_unsigned(rect.width),
            bottom: rect.y.saturating_add_unsigned(rect.height),
        };
        if node.rect == next {
            return;
        }
        let translated = node.dpi == windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd)
            && node.rect.right - node.rect.left == next.right - next.left
            && node.rect.bottom - node.rect.top == next.bottom - next.top;
        node.rect = next;
        translated
    } else {
        return;
    };
    refresh_geometry(&ui, translated);
}

pub(in crate::desktop) unsafe fn move_frame(
    binding: &Binding,
    host: HWND,
    frame: HWND,
    x: i32,
    y: i32,
    attached: &[(HWND, RECT)],
) -> Result<()> {
    let ui = binding.borrow().upgrade();
    movement::cached(ui.as_deref(), host, frame, x, y, attached)
}

unsafe fn refresh_geometry(ui: &Ui, translated: bool) {
    if let Err(error) = window::refresh_at(ui, translated) {
        window::hide(ui);
        ui.fail(error);
        return;
    }
    // 系统拖动的嵌套消息泵可能持续优先处理位置消息，不能等空闲 WM_PAINT 才补框。
    // 几何与所有端点条先更新完，释放借用后在同一次移动内完成可见叠层重绘。
    // handles 首项是 WS_EX_TRANSPARENT 线层；必须最后画，不能让它等仍失效的端点条。
    for overlay in ui
        .handles()
        .into_iter()
        .rev()
        .filter(|h| IsWindowVisible(*h).as_bool())
    {
        let _ = windows::Win32::Graphics::Gdi::UpdateWindow(overlay);
    }
}
