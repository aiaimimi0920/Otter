//! 线程内端点焦点与单槽键盘连线；只SetFocus自有条，不激活其他应用或注入输入。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[path = "workspace_chrome_keyboard_model.rs"]
mod keys;
use keys::Action;

struct Focus {
    port: Port,
    catalog: Rc<Catalog>,
    origin: Node,
}
#[derive(Default)]
pub(super) struct Control {
    focus: RefCell<Option<Rc<Focus>>>,
    source: Cell<Option<Port>>,
    releases: Cell<u32>,
}
impl Control {
    pub fn focused(&self) -> Option<Port> {
        self.focus.borrow().as_ref().map(|focus| focus.port)
    }
    pub fn source(&self) -> Option<Port> {
        self.source.get()
    }
    pub fn cancel_link(&self) {
        self.source.set(None);
    }
    pub fn retire(&self) {
        self.focus.borrow_mut().take();
        self.cancel_link();
    }
    pub fn suppress(&self, key: usize) {
        self.releases.set(self.releases.get() | keys::key_bit(key));
    }
}
unsafe fn strip_owned(ui: &Ui, hwnd: HWND) -> bool {
    IsWindow(Some(hwnd)).as_bool()
        && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == ui as *const Ui as isize
}
unsafe fn strip(ui: &Ui, port: Port) -> Option<HWND> {
    ui.input_windows
        .borrow()
        .iter()
        .find_map(|(hwnd, node, side)| {
            (*node == port.node && *side == port.side && strip_owned(ui, *hwnd)).then_some(*hwnd)
        })
}
unsafe fn current(ui: &Ui, port: Port, catalog: &Rc<Catalog>) -> bool {
    !ui.blocked()
        && ui
            .scene
            .borrow()
            .as_ref()
            .is_some_and(|s| Rc::ptr_eq(&s.catalog, catalog))
        && ui.node(port.node).is_some_and(|node| {
            node.interactive
                && node.ports(ui.mode.get(), false)
                && node.point(port.side, port.index).is_some()
                && IsWindowVisible(HWND(node.hwnd as *mut _)).as_bool()
                && GetWindowLongPtrW(HWND(node.hwnd as *mut _), GWLP_USERDATA) == node.marker
        })
}
unsafe fn available(ui: &Ui, catalog: &Rc<Catalog>) -> Vec<Port> {
    draw::ports(ui)
        .into_iter()
        .map(|(port, _, _)| port)
        .filter(|port| current(ui, *port, catalog))
        .filter(|port| strip(ui, *port).is_some_and(|hwnd| IsWindowVisible(hwnd).as_bool()))
        .collect()
}
unsafe fn focus(ui: &Ui, port: Port, catalog: Rc<Catalog>, origin: Node) -> bool {
    let Some(hwnd) = strip(ui, port).filter(|hwnd| IsWindowVisible(*hwnd).as_bool()) else {
        return false;
    };
    if !current(ui, port, &catalog) {
        return false;
    }
    capsule::cancel(ui);
    if ui.capsule.borrow().is_some() {
        return false;
    }
    let candidate = Rc::new(Focus {
        port,
        catalog,
        origin,
    });
    *ui.keyboard.focus.borrow_mut() = Some(candidate.clone());
    // SetFocus可同步调用旧/新窗口过程；不跨Win32持RefCell借用，返回后核同一候选。
    let _ = SetFocus(Some(hwnd));
    let same = ui
        .keyboard
        .focus
        .borrow()
        .as_ref()
        .is_some_and(|f| Rc::ptr_eq(f, &candidate));
    if !same {
        return false;
    }
    if GetFocus() != hwnd || !current(ui, port, &candidate.catalog) {
        ui.keyboard.retire();
        return false;
    }
    if let Err(error) = window::refresh(ui) {
        window::hide(ui);
        ui.fail(error);
        return false;
    }
    true
}
pub(super) unsafe fn restore(ui: &Ui, port: Port) {
    let scene = ui.scene.borrow().clone();
    if let (Some(scene), Some(origin)) = (scene, ui.node(port.node)) {
        focus(ui, port, scene.catalog.clone(), origin);
    }
}
pub(super) unsafe fn reconcile(ui: &Ui) {
    let focus = ui.keyboard.focus.borrow().clone();
    if focus.is_some_and(|f| {
        !current(ui, f.port, &f.catalog)
            || ui
                .keyboard
                .source()
                .is_some_and(|p| !current(ui, p, &f.catalog))
    }) {
        ui.keyboard.retire();
    }
}
pub(super) unsafe fn lost(ui: &Ui, next: HWND) {
    if !ui
        .input_windows
        .borrow()
        .iter()
        .any(|(hwnd, _, _)| *hwnd == next)
    {
        ui.keyboard.retire();
        ui.repaint();
    }
}
unsafe fn leave(ui: &Ui, origin: &Node) {
    ui.keyboard.retire();
    let host = HWND(origin.hwnd as *mut _);
    if !ui.blocked()
        && IsWindowVisible(host).as_bool()
        && GetWindowLongPtrW(host, GWLP_USERDATA) == origin.marker
        && ui
            .nodes
            .borrow()
            .iter()
            .any(|n| n.hwnd == origin.hwnd && n.marker == origin.marker && n.interactive)
    {
        let _ = SetFocus(Some(host));
    }
    if let Err(error) = window::refresh(ui) {
        window::hide(ui);
        ui.fail(error);
    }
}
unsafe fn link(ui: &Ui, focus: &Focus) {
    if ui.pending.borrow().is_some() || ui.press.borrow().is_some() {
        return;
    }
    let Some(scene) = ui
        .scene
        .borrow()
        .clone()
        .filter(|s| Rc::ptr_eq(&s.catalog, &focus.catalog))
    else {
        return;
    };
    let port = focus.port;
    if let Some(source) = ui
        .keyboard
        .source()
        .filter(|p| current(ui, *p, &focus.catalog))
    {
        if port.side == Side::Output {
            ui.keyboard.source.set(Some(port));
        } else if source.node != port.node {
            match scene
                .catalog
                .make_link(source.node, port.node, source.index, port.index)
            {
                Ok(Some(link))
                    if PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0)).is_ok() =>
                {
                    *ui.pending.borrow_mut() = Some(desktop::link_catalog::Creation {
                        catalog: focus.catalog.clone(),
                        link,
                    });
                    ui.keyboard.cancel_link();
                }
                Err(error) => eprintln!("native-workspace-link: {error}"),
                _ => (),
            }
        }
    } else {
        ui.keyboard.source.set(if port.side == Side::Output {
            Some(port)
        } else {
            scene
                .edges
                .iter()
                .find(|edge| edge.to == port)
                .map(|edge| edge.from)
                .filter(|p| current(ui, *p, &focus.catalog))
        });
    }
    ui.repaint();
}
pub(super) unsafe fn translate(ui: &Ui, message: &MSG) -> bool {
    if !matches!(message.message, WM_KEYDOWN | WM_KEYUP) {
        return false;
    }
    let input = strip_owned(ui, message.hwnd)
        && ui
            .input_windows
            .borrow()
            .iter()
            .any(|(hwnd, _, _)| *hwnd == message.hwnd);
    let host = ui
        .nodes
        .borrow()
        .iter()
        .find(|node| {
            node.hwnd == message.hwnd.0 as isize
                && node.interactive
                && GetWindowLongPtrW(message.hwnd, GWLP_USERDATA) == node.marker
        })
        .cloned();
    if (!input && host.is_none()) || GetFocus() != message.hwnd {
        return false;
    }
    let bit = keys::key_bit(message.wParam.0);
    if message.message == WM_KEYUP {
        let held = ui.keyboard.releases.get();
        ui.keyboard.releases.set(held & !bit);
        return held & bit != 0;
    }
    reconcile(ui);
    if message.lParam.0 & (1 << 30) != 0
        && ui.keyboard.releases.get() & bit != 0
        && (ui.keyboard.focused().is_none() || ui.pending.borrow().is_some() || ui.blocked())
    {
        // Esc/F6回到宿主后，同一次长按仍由端点条消费，不能落入文档删除或Live源输入。
        return true;
    }
    if ui.blocked() || ui.press.borrow().is_some() || ui.pending.borrow().is_some() {
        return false;
    }
    let Some(action) = keys::action(
        message.wParam.0,
        GetKeyState(VK_CONTROL.0 as i32) < 0,
        GetKeyState(VK_SHIFT.0 as i32) < 0,
        GetKeyState(VK_MENU.0 as i32) < 0,
        GetKeyState(VK_LWIN.0 as i32) < 0 || GetKeyState(VK_RWIN.0 as i32) < 0,
    ) else {
        return false;
    };
    let focus_state = ui.keyboard.focus.borrow().clone();
    if let Some(f) = focus_state
        .as_ref()
        .filter(|f| input && strip(ui, f.port) == Some(message.hwnd))
    {
        ui.keyboard.releases.set(ui.keyboard.releases.get() | bit);
        if message.lParam.0 & (1 << 30) != 0 && !matches!(action, Action::Move(_) | Action::Edge(_))
        {
            return true;
        }
        match action {
            Action::EnterRail(_) => leave(ui, &f.origin),
            Action::Escape => {
                if ui.keyboard.source().is_some() {
                    ui.keyboard.cancel_link();
                    ui.repaint();
                } else {
                    leave(ui, &f.origin);
                }
            }
            Action::Link => link(ui, f),
            Action::Edit => {
                capsule::request(ui, f.port, true);
            }
            Action::Disclose => {
                let mut disclosed = ui.disclosed.borrow_mut();
                if !disclosed.remove(&f.port) {
                    disclosed.insert(f.port);
                }
                let expanded = disclosed.contains(&f.port);
                drop(disclosed);
                if expanded {
                    capsule::request(ui, f.port, false);
                } else {
                    capsule::cancel(ui);
                }
                if let Err(error) = window::refresh(ui) {
                    window::hide(ui);
                    ui.fail(error);
                }
            }
            Action::Move(backward) | Action::Edge(backward) => {
                let ports = available(ui, &f.catalog);
                let next = if matches!(action, Action::Edge(_)) {
                    if backward {
                        ports.len().checked_sub(1)
                    } else {
                        Some(0)
                    }
                } else {
                    ports
                        .iter()
                        .position(|p| *p == f.port)
                        .and_then(|i| keys::position(i, ports.len(), backward))
                };
                if let Some(port) = next.and_then(|i| ports.get(i)).copied() {
                    focus(ui, port, f.catalog.clone(), f.origin.clone());
                }
            }
        }
        return true;
    }
    let Action::EnterRail(backward) = action else {
        return false;
    };
    if !input && host.is_none() {
        return false;
    }
    let Some(scene) = ui.scene.borrow().clone() else {
        return false;
    };
    let ports = available(ui, &scene.catalog);
    let endpoint = host.as_ref().map(|n| n.endpoint).or_else(|| {
        ui.input_windows
            .borrow()
            .iter()
            .find(|(h, _, _)| *h == message.hwnd)
            .map(|(_, n, _)| *n)
    });
    let mut matching = ports.into_iter().filter(|p| Some(p.node) == endpoint);
    let port = if backward {
        matching.next_back()
    } else {
        matching.next()
    };
    let Some(port) = port else {
        return false;
    };
    ui.keyboard.releases.set(ui.keyboard.releases.get() | bit);
    if message.lParam.0 & (1 << 30) == 0 {
        if let Some(origin) = ui.node(port.node) {
            focus(ui, port, scene.catalog.clone(), origin);
        }
    }
    true
}
#[cfg(test)]
#[path = "workspace_chrome_keyboard_tests.rs"]
mod tests;
