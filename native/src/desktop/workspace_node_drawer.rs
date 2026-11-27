//! Application owner of per-document bottom settings strips; independent expansion is not persisted, and actions reuse parameter-commit transactions.
use super::*;
use desktop::{
    configuration_stamp::Stamp, document_window::parameter_panel, node_settings::Action,
};
use neuro_mini_ui::{Drawer, DrawerButton, Icon, DRAWER_TOGGLE};
#[path = "workspace_node_drawer_window.rs"]
mod host;
#[path = "workspace_node_drawer_model.rs"]
mod projection;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "workspace_node_drawer_tests.rs"]
mod tests;
const NOTIFY: u32 = WM_APP + 0x674;
const RUN: u16 = 20;
#[derive(Clone, PartialEq)]
struct Model {
    stamp: Stamp,
    language: crate::ui_text::DisplayLanguage,
    settings: crate::unit_node_settings::Settings,
    title: String,
    art: bool,
    has_execution: bool,
    enabled: bool,
    run: bool,
    checked: [bool; 5],
}
struct Data {
    chrome: Weak<Ui>,
    hwnd: Cell<HWND>,
    host: HWND,
    marker: isize,
    model: RefCell<Model>,
    context: Cell<u32>,
    open: Cell<bool>,
    tab_expanded: Cell<bool>,
    pending: Cell<Option<(u32, u16)>>,
    root: Cell<HWND>,
    size: Cell<(i32, i32)>,
    presentation: Cell<Option<(bool, u32)>>,
}
impl Data {
    fn advance(&self) -> Result<()> {
        self.pending.set(None);
        self.context.set(
            self.context
                .get()
                .checked_add(1)
                .ok_or_else(|| Error::from(E_UNEXPECTED))?,
        );
        Ok(())
    }
    fn set_open(&self, open: bool) -> Result<()> {
        if self.open.replace(open) != open {
            self.advance()?;
        }
        Ok(())
    }
}
pub(super) struct Open {
    // Release child/top-level windows before callback data; Data holds only Weak references to chrome, preventing Rc cycles.
    drawer: Drawer,
    window: host::Window,
    data: Rc<Data>,
}
impl Open {
    unsafe fn new(ui: &Rc<Ui>, node: &Node, model: Model) -> Result<Rc<Self>> {
        let data = Rc::new(Data {
            chrome: Rc::downgrade(ui),
            hwnd: Cell::new(HWND::default()),
            host: HWND(node.hwnd as *mut _),
            marker: node.marker,
            model: RefCell::new(model.clone()),
            context: Cell::new(1),
            open: Cell::new(node.expanded),
            tab_expanded: Cell::new(node.expanded),
            pending: Cell::new(None),
            root: Cell::new(HWND::default()),
            size: Cell::new((20, 10)),
            presentation: Cell::new(None),
        });
        let window = host::Window::new(&data)?;
        let drawer = Drawer::new(window.0, NOTIFY, &projection::buttons(&model))?;
        data.root.set(drawer.handle());
        Ok(Rc::new(Self {
            drawer,
            window,
            data,
        }))
    }
    fn matches(&self, node: &Node) -> bool {
        self.data.host.0 as isize == node.hwnd
            && self.data.marker == node.marker
            && unsafe { self.alive() }
    }
    unsafe fn alive(&self) -> bool {
        IsWindow(Some(self.window.0)).as_bool()
            && GetWindowLongPtrW(self.window.0, GWLP_USERDATA) == Rc::as_ptr(&self.data) as isize
            && self.drawer.alive()
    }
    unsafe fn update(&self, node: &Node, model: Model) -> Result<()> {
        if windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(self.window.0).as_bool()
            != model.enabled
        {
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                self.window.0,
                model.enabled,
            );
        }
        let changed = *self.data.model.borrow() != model;
        if changed {
            SetWindowTextW(
                self.window.0,
                &HSTRING::from(crate::ui_text::text("workspace-node-properties")),
            )?;
            *self.data.model.borrow_mut() = model;
            self.data.presentation.set(None);
            self.data.advance()?;
        }
        let model = self.data.model.borrow().clone();
        let open = self.data.open.get();
        let changed = self.data.presentation.get() != Some((open, node.dpi));
        let visible = IsWindowVisible(self.window.0).as_bool();
        // Top-level windows also retain old frames until children, triangle clipping, and final centering all finish.
        // Do not freeze hosts about to hide: WM_SETREDRAW visibility restoration must not overwrite final hidden state.
        let held = changed && visible && self.shown(node);
        let redraw = held.then(|| neuro_mini_ui::paint_batch::Hold::new(self.window.0));
        let layout = self.drawer.update(
            &model.title,
            &projection::buttons(&model),
            open,
            self.data.context.get(),
            node.dpi,
        )?;
        self.data.size.set((layout.width, layout.height));
        if changed {
            layout.apply_region(self.window.0, open, node.dpi)?;
            self.data.presentation.set(Some((open, node.dpi)));
        }
        self.place_with(node, held)?;
        drop(redraw);
        Ok(())
    }
    unsafe fn place(&self, node: &Node) -> Result<()> {
        self.place_with(node, false)
    }
    fn shown(&self, node: &Node) -> bool {
        node.rect.right > node.rect.left
            && node.rect.bottom > node.rect.top
            && !node.minified
            && (self.data.tab_expanded.get()
                || self
                    .data
                    .chrome
                    .upgrade()
                    .is_some_and(|ui| ui.selected.borrow().contains(&(node.hwnd, node.marker))))
            && (self.data.open.get() || !node.hide_collapsed_endpoints)
    }
    unsafe fn place_with(&self, node: &Node, held: bool) -> Result<()> {
        let (width, height) = self.data.size.get();
        let shown = self.shown(node);
        let desired = RECT {
            left: node
                .rect
                .left
                .saturating_add((node.rect.right - node.rect.left - width) / 2),
            top: node.rect.bottom,
            right: 0,
            bottom: 0,
        };
        let mut current = RECT::default();
        GetWindowRect(self.window.0, &mut current)?;
        if current.left != desired.left
            || current.top != desired.top
            || current.right - current.left != width
            || current.bottom - current.top != height
            || (held || IsWindowVisible(self.window.0).as_bool()) != shown
        {
            if shown {
                if let Some(ui) = self.data.chrome.upgrade() {
                    window::order_drawing(&ui)?;
                }
            }
            SetWindowPos(
                self.window.0,
                None,
                desired.left,
                desired.top,
                width,
                height,
                SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOOWNERZORDER
                    | if held {
                        SWP_NOREDRAW
                    } else if shown {
                        SWP_SHOWWINDOW
                    } else {
                        SWP_HIDEWINDOW
                    },
            )?;
        }
        Ok(())
    }
}
pub(super) unsafe fn sync(app: &mut Application) -> Result<()> {
    let ui = app.chrome.ui.clone();
    let nodes = ui.nodes.borrow().clone();
    let selected = ui.selected.borrow().clone();
    let previous = ui.drawers.borrow().clone();
    let mut next = Vec::new();
    let mut restored_nodes = Vec::new();
    for doc in &app.documents {
        let Some(node) = nodes.iter().find(|n| {
            n.hwnd == doc.window.0 .0 as isize && n.marker == (&*doc.state as *const State) as isize
        }) else {
            continue;
        };
        let restored = doc
            .state
            .click
            .chrome
            .borrow()
            .as_ref()
            .and_then(|s| s.drawer);
        if node.minified
            || !(restored.is_some()
                || node.expanded
                || selected.contains(&(node.hwnd, node.marker))
                || previous
                    .iter()
                    .any(|o| o.matches(node) && o.data.tab_expanded.get()))
        {
            continue;
        }
        let node = node.clone();
        let mut model = projection::capture(&doc.state);
        let open = match previous
            .iter()
            .find(|o| o.matches(&node) && o.data.model.borrow().art == model.art)
        {
            Some(o) => o.clone(),
            None => Open::new(&ui, &node, model.clone())?,
        };
        if let Some((context, id)) = open
            .data
            .pending
            .take()
            .filter(|(context, _)| *context == open.data.context.get())
        {
            let old = open.data.model.borrow().clone();
            if old == model && model.enabled && context != 0 {
                if id == DRAWER_TOGGLE {
                    open.data.set_open(!open.data.open.get())?;
                } else if let Some(action) = projection::intent(id, &old) {
                    if let Some(panel) = doc.parameters.as_ref().filter(|panel| !panel.retired()) {
                        panel.queue_node_action(old.stamp.clone(), action);
                    } else {
                        match action {
                            Some(action) => {
                                desktop::node_settings::apply(doc.window.0, &doc.state, action)?
                            }
                            None => desktop::execution_start::request(doc.window.0, &doc.state)?,
                        }
                        // Direct execution may synchronously change documents; no-action paths retain the original snapshot.
                        model = projection::capture(&doc.state);
                    }
                }
            }
        }
        if let Some((expanded, tab)) = restored {
            open.data.tab_expanded.set(tab);
            open.data.set_open(expanded)?;
            restored_nodes.push(&doc.state);
        }
        open.update(&node, model)?;
        next.push(open);
    }
    *ui.drawers.borrow_mut() = next;
    window::order_drawing(&ui)?;
    for s in restored_nodes {
        if !s.click.active() {
            s.click.chrome.take();
        }
    }
    Ok(())
}
pub(super) fn snapshot(ui: &Ui, n: &Node) -> Option<(bool, bool)> {
    ui.drawers
        .borrow()
        .iter()
        .find(|o| o.matches(n))
        .map(|o| (o.data.open.get(), o.data.tab_expanded.get()))
}
/// Only the actual master Tab toggle synchronizes bottom drawers; collapsing endpoints via left properties is not a Tab action.
pub(in crate::desktop::application) unsafe fn tab(binding: &Binding, host: HWND, expanded: bool) {
    if let Some(ui) = binding.borrow().upgrade() {
        for open in ui.drawers.borrow().iter().filter(|o| o.data.host == host) {
            if GetWindowLongPtrW(host, GWLP_USERDATA) == open.data.marker {
                open.data.tab_expanded.set(expanded);
                if let Err(error) = open.data.set_open(expanded) {
                    ui.fail(error);
                }
            }
        }
    }
}
pub(super) fn handles(ui: &Ui) -> Vec<HWND> {
    // Forcibly destroyed old owners may briefly coexist with rebuilt ones; retired handles must not enter Z-order batches.
    ui.drawers
        .borrow()
        .iter()
        .filter(|o| unsafe { o.alive() })
        .map(|o| o.window.0)
        .collect()
}
pub(super) fn moving(ui: &Ui, host: HWND) -> Vec<HWND> {
    ui.drawers
        .borrow()
        .iter()
        .filter(|o| o.data.host == host && unsafe { o.alive() })
        .map(|o| o.window.0)
        .collect()
}
pub(super) fn idle(ui: &Ui) -> bool {
    ui.drawers
        .borrow()
        .iter()
        .all(|o| o.data.pending.get().is_none())
}
pub(super) fn cancel(ui: &Ui) {
    for open in ui.drawers.borrow().iter() {
        open.data.pending.set(None);
        if unsafe { IsWindowVisible(open.window.0).as_bool() } {
            if let Some(next) = open.data.context.get().checked_add(1) {
                open.data.context.set(next);
            } else {
                ui.fail(Error::from(E_UNEXPECTED));
            }
        }
    }
}
pub(super) unsafe fn geometry(ui: &Ui) -> Result<()> {
    let owners = ui.drawers.borrow().clone();
    for open in owners {
        if let Some(node) = ui.nodes.borrow().iter().find(|n| open.matches(n)).cloned() {
            open.place(&node)?;
        }
    }
    Ok(())
}
pub(super) unsafe fn translate(ui: &Ui, msg: &MSG) -> bool {
    let owners = ui.drawers.borrow().clone();
    for open in owners {
        if msg.hwnd == open.window.0 || IsChild(open.window.0, msg.hwnd).as_bool() {
            if ui.blocked() {
                return false;
            }
            // Attached strips are not independent documents; edit-strip shortcuts still go to original hosts, while Tab retains standard control navigation.
            if msg.message == WM_KEYDOWN
                && (msg.wParam.0 == windows::Win32::UI::Input::KeyboardAndMouse::VK_F4.0 as usize
                    || (msg.wParam.0 == 0x45
                        && windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x11) < 0
                        && windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x10) >= 0
                        && windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x12) >= 0))
                && GetWindowLongPtrW(open.data.host, GWLP_USERDATA) == open.data.marker
            {
                SendMessageW(
                    open.data.host,
                    msg.message,
                    Some(msg.wParam),
                    Some(msg.lParam),
                );
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(open.data.host));
                return true;
            }
            if msg.message == WM_KEYDOWN
                && msg.wParam.0 == windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE.0 as usize
            {
                if let Err(error) = open.data.set_open(false) {
                    ui.fail(error);
                }
                open.data.pending.set(None);
                let _ = PostMessageW(Some(open.data.host), WM_NULL, WPARAM(0), LPARAM(0));
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(open.data.host));
                return true;
            }
            return open.drawer.translate(msg);
        }
    }
    false
}
