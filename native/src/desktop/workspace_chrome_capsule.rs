//! 工作区仅一个活动单字段child，复用参数面板控件/事务；不为每端口建窗或复制业务值。
use super::*;
use desktop::document_window::parameter_panel::Panel;
use windows::Win32::{Graphics::Gdi::*, UI::Input::KeyboardAndMouse::*};

pub(super) struct Open {
    port: Port,
    catalog: Rc<Catalog>,
    host: isize,
    marker: isize,
    parent: HWND,
    field: String,
    panel: RefCell<Option<Rc<Panel>>>,
    height: Cell<i32>,
    enter: Cell<bool>,
    entered: Cell<bool>,
}
pub(super) struct Resume {
    unit: crate::workspace_id::UnitId,
    field: String,
    host: isize,
    marker: isize,
    focus: bool,
}
fn selected(ui: &Ui) -> Option<Rc<Open>> {
    ui.capsule.borrow().clone()
}
fn same(ui: &Ui, open: &Rc<Open>) -> bool {
    selected(ui).is_some_and(|current| Rc::ptr_eq(&current, open))
}
unsafe fn current(ui: &Ui, open: &Open) -> bool {
    !ui.blocked()
        && ui.press.borrow().is_none()
        && ui.pending.borrow().is_none()
        && ui.keyboard.source().is_none()
        && ui
            .scene
            .borrow()
            .as_ref()
            .is_some_and(|s| Rc::ptr_eq(&s.catalog, &open.catalog))
        && ui.node(open.port.node).is_some_and(|node| {
            node.hwnd == open.host
                && node.marker == open.marker
                && node.interactive
                && node.ports(ui.mode.get(), false)
                && !node.minified
                && (node.expanded || ui.disclosed.borrow().contains(&open.port))
                && node.point(open.port.side, open.port.index).is_some()
                && IsWindowVisible(HWND(open.host as *mut _)).as_bool()
                && GetWindowLongPtrW(HWND(open.host as *mut _), GWLP_USERDATA) == open.marker
        })
        && IsWindow(Some(open.parent)).as_bool()
        && GetWindowLongPtrW(open.parent, GWLP_USERDATA) == ui as *const Ui as isize
}
pub(super) unsafe fn cancel(ui: &Ui) {
    let old = ui.capsule.borrow_mut().take();
    if let Some(panel) = old.as_ref().and_then(|open| open.panel.borrow().clone()) {
        panel.retire_embedded();
    }
    drop(old); // 同步失焦/销毁前已撤槽，不跨Win32持借用。
}
unsafe fn cancel_current(ui: &Ui, open: &Rc<Open>) {
    if same(ui, open) {
        cancel(ui);
    }
}
unsafe fn restore(ui: &Ui, open: &Rc<Open>) {
    if same(ui, open) {
        cancel_current(ui, open);
        // 退休也可同步安装后继；旧栈不得借恢复焦点再次取消它。
        if selected(ui).is_none() {
            keyboard::restore(ui, open.port);
        }
    }
}
pub(super) fn idle(ui: &Ui) -> bool {
    selected(ui).is_none_or(|open| {
        open.panel
            .borrow()
            .as_ref()
            .is_some_and(|panel| unsafe { panel.exit_idle() })
    })
}
pub(super) unsafe fn reconcile(ui: &Ui) {
    if selected(ui).is_some_and(|open| !current(ui, &open)) {
        cancel(ui);
    }
}
pub(super) unsafe fn request(ui: &Ui, port: Port, enter: bool) {
    if port.side != Side::Input || ui.blocked() {
        return;
    }
    if let Some(open) = selected(ui).filter(|open| open.port == port && current(ui, open)) {
        open.enter.set(open.enter.get() || enter);
        if let Some(scene) = ui.scene.borrow().clone() {
            let _ = PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0));
        }
        return;
    }
    cancel(ui);
    if selected(ui).is_some() {
        return;
    }
    let Some(scene) = ui.scene.borrow().clone() else {
        return;
    };
    let Some(node) = ui.node(port.node).filter(|n| n.interactive && !n.minified) else {
        return;
    };
    let Some(field) = scene
        .catalog
        .endpoints
        .get(port.node)
        .and_then(|e| e.inputs.get(port.index))
        .cloned()
    else {
        return;
    };
    let Some(parent) = ui
        .input_windows
        .borrow()
        .iter()
        .find_map(|(h, n, s)| (*n == port.node && *s == Side::Input).then_some(*h))
    else {
        return;
    };
    ui.disclosed.borrow_mut().insert(port);
    let open = Rc::new(Open {
        port,
        catalog: scene.catalog.clone(),
        host: node.hwnd,
        marker: node.marker,
        parent,
        field,
        panel: RefCell::default(),
        height: Cell::new(240),
        enter: Cell::new(enter),
        entered: Cell::new(false),
    });
    if current(ui, &open) && PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0)).is_ok() {
        *ui.capsule.borrow_mut() = Some(open);
    }
}
/// Child区域始终在宿主图片外；左侧不足则翻到右侧，空间不足拒绝而非盖住图片。
pub(super) unsafe fn area(ui: &Ui, parent: HWND) -> Option<RECT> {
    let open = selected(ui).filter(|open| open.parent == parent && current(ui, open))?;
    let node = ui.node(open.port.node)?;
    let point = node.point(Side::Input, open.port.index)?;
    let scale = |n: i32| n * node.dpi.clamp(96, 768) as i32 / 96;
    let (width, height) = (scale(300), scale(open.height.get()));
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    GetMonitorInfoW(
        MonitorFromWindow(HWND(open.host as *mut _), MONITOR_DEFAULTTONEAREST),
        &mut info,
    )
    .ok()
    .ok()?;
    let work = info.rcWork;
    let left = node
        .rect
        .left
        .saturating_sub(node.radius())
        .saturating_sub(width);
    let right = node.rect.right.saturating_add(node.radius());
    let x = if left >= work.left {
        left
    } else if right.checked_add(width)? <= work.right {
        right
    } else {
        return None;
    };
    if height > work.bottom.checked_sub(work.top)? {
        return None;
    }
    let y = point
        .y
        .saturating_sub(node.radius())
        .clamp(work.top, work.bottom - height);
    Some(RECT {
        left: x,
        top: y,
        right: x.checked_add(width)?,
        bottom: y.checked_add(height)?,
    })
}
pub(super) unsafe fn place(ui: &Ui, parent: HWND, bounds: RECT) -> Result<()> {
    let Some(open) = selected(ui).filter(|open| open.parent == parent && current(ui, open)) else {
        return Ok(());
    };
    let panel = open.panel.borrow().clone();
    if let Some(panel) = panel {
        let Some(rect) = area(ui, parent) else {
            cancel(ui);
            return Ok(());
        };
        let result = panel.place_embedded(RECT {
            left: rect.left - bounds.left,
            right: rect.right - bounds.left,
            top: rect.top - bounds.top,
            bottom: rect.bottom - bounds.top,
        });
        if !same(ui, &open) {
            return Ok(());
        }
        if !current(ui, &open) {
            cancel_current(ui, &open);
            return Ok(());
        }
        result?;
    }
    Ok(())
}
/// 所有意图在Application原调度段消费。只有本owner已确认的提交可重建后继显示，不继承草稿。
pub(super) unsafe fn drain(app: &Application) -> Option<Resume> {
    let ui = &app.chrome.ui;
    let open = selected(ui)?;
    if !current(ui, &open)
        || !open
            .catalog
            .matches(&app.documents, &app.live_sessions, &app.workspace)
    {
        cancel(ui);
        return None;
    }
    let panel = open.panel.borrow().clone()?;
    if panel.retired() {
        cancel(ui);
        return None;
    }
    let focus = panel.owns_focus();
    if (open.entered.get() || !panel.exit_idle()) && !focus && GetFocus() != open.parent {
        cancel(ui);
        return None;
    }
    open.entered.set(open.entered.get() || focus);
    let doc = app
        .documents
        .iter()
        .find(|doc| doc.window.0 .0 as isize == open.host && doc.is_alive())?;
    let topology = app.workspace.topology_stamp();
    let appearance = app.workspace.appearance_stamp();
    let before = desktop::configuration_stamp::Stamp::capture(&doc.state);
    panel.take_commit();
    let result = panel.update(&doc.state);
    if !same(ui, &open) {
        return None;
    }
    if let Err(error) = result {
        cancel_current(ui, &open);
        ui.fail(error);
        return None;
    }
    let committed = panel.take_commit();
    if before != desktop::configuration_stamp::Stamp::capture(&doc.state) {
        let resume = (committed
            && same(ui, &open)
            && topology.matches(&app.workspace)
            && appearance.matches(&app.workspace)
            && current(ui, &open)
            && doc.is_alive())
        .then(|| Resume {
            unit: doc.state.unit_id.clone(),
            field: open.field.clone(),
            host: open.host,
            marker: open.marker,
            focus: focus && panel.owns_focus(),
        });
        cancel_current(ui, &open);
        return resume.filter(|_| selected(ui).is_none());
    }
    None
}
pub(super) unsafe fn resume(ui: &Ui, resume: Resume) {
    if selected(ui).is_some() {
        return;
    }
    let scene = ui.scene.borrow().clone();
    let port = scene.and_then(|scene| {
        let node = scene
            .catalog
            .endpoints
            .iter()
            .position(|e| e.unit == resume.unit)?;
        let index = scene.catalog.endpoints[node]
            .inputs
            .iter()
            .position(|p| p == &resume.field)?;
        ui.node(node)
            .filter(|n| n.hwnd == resume.host && n.marker == resume.marker)?;
        Some(Port {
            node,
            side: Side::Input,
            index,
        })
    });
    if let Some(port) = port {
        request(ui, port, resume.focus);
    }
}
pub(super) unsafe fn mount(app: &Application) {
    let ui = &app.chrome.ui;
    let Some(open) = selected(ui).filter(|open| current(ui, open)) else {
        return;
    };
    let mut changed = false;
    if open.panel.borrow().is_none() {
        let Some(doc) = app
            .documents
            .iter()
            .find(|doc| doc.window.0 .0 as isize == open.host && doc.is_alive())
        else {
            cancel(ui);
            return;
        };
        let model = desktop::capability_parameter_menu::Snapshot::capture(&doc.state);
        if (doc.state.is_art() || open.field != "image")
            && !model.is_some_and(|s| s.choices.iter().any(|c| c.id == open.field))
        {
            cancel(ui);
            return;
        }
        match Panel::embedded(doc.window.0, &doc.state, open.parent, &open.field) {
            Ok(panel) if same(ui, &open) && current(ui, &open) => {
                open.height.set(panel.embedded_height());
                *open.panel.borrow_mut() = Some(Rc::new(panel));
                changed = true;
            }
            Ok(_) => {
                cancel_current(ui, &open);
                return;
            }
            Err(error) => {
                if same(ui, &open) {
                    cancel_current(ui, &open);
                    ui.fail(error);
                }
                return;
            }
        }
    }
    let enter = open.enter.replace(false);
    if changed || enter {
        if let Err(error) = window::refresh(ui) {
            if same(ui, &open) {
                cancel_current(ui, &open);
                ui.fail(error);
            }
            return;
        }
    }
    if enter
        && same(ui, &open)
        && current(ui, &open)
        && (GetFocus() == open.parent || GetFocus() == HWND(open.host as *mut _))
    {
        let panel = open.panel.borrow().clone();
        if let Some(panel) = panel {
            panel.focus_embedded();
            open.entered.set(panel.owns_focus());
        }
        if same(ui, &open) && !current(ui, &open) {
            cancel_current(ui, &open);
        }
    }
}
pub(super) unsafe fn translate(ui: &Ui, message: &MSG) -> bool {
    let Some(open) = selected(ui) else {
        return false;
    };
    let panel = open.panel.borrow().clone();
    let Some(panel) =
        panel.filter(|p| message.hwnd == p.handle() || IsChild(p.handle(), message.hwnd).as_bool())
    else {
        return false;
    };
    if !current(ui, &open) {
        cancel(ui);
        return true;
    }
    if message.message == WM_KEYDOWN && message.wParam.0 == VK_F6.0 as usize {
        if message.lParam.0 & (1 << 30) == 0
            && !desktop::document_window::parameter_panel::composing_focus()
        {
            SendMessageW(panel.handle(), WM_CLOSE, None, None);
            restore(ui, &open);
        }
        ui.keyboard.suppress(message.wParam.0);
        return true;
    }
    let handled = panel.translate(message);
    if handled && message.message == WM_KEYDOWN {
        ui.keyboard.suppress(message.wParam.0);
    }
    if panel.retired() {
        restore(ui, &open);
    }
    handled
}
#[cfg(test)]
#[path = "workspace_chrome_capsule_tests.rs"]
mod tests;
