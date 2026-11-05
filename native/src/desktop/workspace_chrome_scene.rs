//! 从已验证 owner 捕获公共端口/几何；不让隐藏组或过期 HWND 留下悬空连线。
use super::*;
pub(super) struct Gates {
    pub modals: desktop::modal_scope::Scope,
    pub session: Rc<desktop::session_end::Control>,
    pub capture: Rc<Cell<bool>>,
    pub exit: Rc<Cell<bool>>,
}
impl Gates {
    pub fn blocked(&self) -> bool {
        self.modals.blocked() || self.session.paused() || self.capture.get() || self.exit.get()
    }
}
pub(super) struct Scene {
    pub catalog: Rc<Catalog>,
    pub edges: Vec<Edge>,
    pub wake: HWND,
}
pub(super) unsafe fn frame(hwnd: HWND) -> Option<Rect> {
    if !IsWindow(Some(hwnd)).as_bool()
        || !IsWindowVisible(hwnd).as_bool()
        || IsIconic(hwnd).as_bool()
    {
        return None;
    }
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect).ok()?;
    let mut origin = POINT::default();
    windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut origin)
        .ok()
        .ok()?;
    let result = Rect {
        left: origin.x,
        top: origin.y,
        right: origin.x.checked_add(rect.right)?,
        bottom: origin.y.checked_add(rect.bottom)?,
    };
    result.valid().then_some(result)
}
impl Scene {
    pub unsafe fn capture(app: &Application, catalog: Rc<Catalog>) -> (Rc<Self>, Vec<Node>) {
        let previous = app.chrome.ui.scene.borrow().clone();
        let cached = app.chrome.ui.nodes.borrow().clone();
        let mut nodes = Vec::new();
        for (endpoint, entry) in catalog.endpoints.iter().enumerate() {
            let (hwnd, minified, expanded, protected) = if let Some(doc) = app
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == entry.unit)
            {
                if !doc.is_alive() || doc.state.group_hidden.get() {
                    continue;
                }
                (
                    doc.window.0,
                    doc.state.canvas.borrow().minified.is_some(),
                    IsWindowVisible(doc.state.parameters_ui.window()).as_bool(),
                    doc.state.capture_protection_requested.get(),
                )
            } else {
                let Some(session) = app.live_sessions.iter().find(|session| {
                    session.unit_id() == &entry.unit && session.belongs_to(&app.workspace)
                }) else {
                    continue;
                };
                let Some(hwnd) = session.group_dialog_owner() else {
                    continue;
                };
                (hwnd, false, false, true)
            };
            let marker = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            // 宿主几何消息直接更新标量；Live 的帧通知不重复读取客户区或重建图。
            let rect = cached
                .iter()
                .find(|node| node.hwnd == hwnd.0 as isize && node.marker == marker)
                .filter(|node| node.rect.valid())
                .filter(|_| IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool())
                .map(|node| node.rect)
                .or_else(|| frame(hwnd));
            let Some(rect) = rect.filter(|rect| rect.valid()) else {
                continue;
            };
            nodes.push(Node {
                endpoint,
                hwnd: hwnd.0 as isize,
                marker,
                rect,
                dpi: windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
                minified,
                expanded,
                protected,
                interactive: !app.link_owner_blocked(&entry.unit),
                inputs: entry.inputs.len(),
                outputs: entry.outputs.len(),
                input_visibility: entry.input_visibility.clone(),
                output_visibility: entry.output_visibility.clone(),
            });
        }
        if let Some(previous) = previous.filter(|old| Rc::ptr_eq(&old.catalog, &catalog)) {
            return (previous, nodes);
        }
        let mut edges = Vec::new();
        for edge in app.workspace.links() {
            let Some(from) = catalog
                .endpoints
                .iter()
                .position(|e| e.unit == edge.from_unit)
            else {
                continue;
            };
            let Some(to) = catalog
                .endpoints
                .iter()
                .position(|e| e.unit == edge.to_unit)
            else {
                continue;
            };
            // 图不随暂时隐藏改变；绘制时才按当前nodes过滤，恢复无需换Catalog。
            let Some(output) = catalog.endpoints[from]
                .outputs
                .iter()
                .position(|p| p == &edge.from_port)
            else {
                continue;
            };
            let Some(input) = catalog.endpoints[to]
                .inputs
                .iter()
                .position(|p| p == &edge.to_port)
            else {
                continue;
            };
            edges.push(Edge {
                from: Port {
                    node: from,
                    side: Side::Output,
                    index: output,
                },
                to: Port {
                    node: to,
                    side: Side::Input,
                    index: input,
                },
            });
        }
        let wake = app
            .documents
            .iter()
            .find(|d| d.is_alive())
            .map_or(HWND::default(), |d| d.window.0);
        (
            Rc::new(Self {
                catalog,
                edges,
                wake,
            }),
            nodes,
        )
    }
}
