//! Restore only the suspended node. Catalog indices and HWNDs may be reused;
//! identity, names and the latest explicit selection take precedence.
use super::*;
pub(in crate::desktop) struct Suspended {
    ui: Weak<Ui>,
    key: (isize, isize),
    epoch: u64,
    selection: Option<usize>,
    expanded: bool,
    ports: Vec<(Side, String, bool)>,
    pub(super) drawer: Option<(bool, bool)>,
}
impl Suspended {
    pub(in crate::desktop) fn capture(state: &State, host: HWND) -> Option<Self> {
        let ui = state.chrome.borrow().upgrade()?;
        let key = (host.0 as isize, state as *const State as isize);
        let nodes = ui.nodes.borrow();
        let n = nodes.iter().find(|n| (n.hwnd, n.marker) == key)?;
        let scene = ui.scene.borrow();
        let entry = scene.as_ref()?.catalog.endpoints.get(n.endpoint)?;
        if entry.unit != state.unit_id {
            return None;
        }
        let mut ports = Vec::new();
        for (&p, &expanded) in ui
            .disclosed
            .borrow()
            .iter()
            .filter(|(p, _)| p.node == n.endpoint)
        {
            let names = match p.side {
                Side::Input => &entry.inputs,
                Side::Output => &entry.outputs,
            };
            if let Some(name) = names.get(p.index) {
                ports.push((p.side, name.clone(), expanded));
            }
        }
        let selection = ui.selected.borrow().iter().position(|k| *k == key);
        Some(Self {
            ui: Rc::downgrade(&ui),
            key,
            epoch: ui.selection_epoch.get(),
            selection,
            expanded: n.expanded,
            ports,
            drawer: drawer::snapshot(&ui, n),
        })
    }
    pub(super) fn restore(
        &self,
        state: &State,
        ui: &Rc<Ui>,
        scene: &scene::Scene,
        n: &Node,
    ) -> bool {
        if !self.ui.ptr_eq(&Rc::downgrade(ui))
            || self.key != (n.hwnd, n.marker)
            || self.expanded != n.expanded
        {
            return false;
        }
        let Some(entry) = scene
            .catalog
            .endpoints
            .get(n.endpoint)
            .filter(|e| e.unit == state.unit_id)
        else {
            return false;
        };
        for (side, name, value) in &self.ports {
            let names = match side {
                Side::Input => &entry.inputs,
                Side::Output => &entry.outputs,
            };
            if let Some(index) = names.iter().position(|s| s == name) {
                ui.disclosed.borrow_mut().insert(
                    Port {
                        node: n.endpoint,
                        side: *side,
                        index,
                    },
                    *value,
                );
            }
        }
        if let Some(index) = self
            .selection
            .filter(|_| self.epoch == ui.selection_epoch.get())
        {
            let mut selected = ui.selected.borrow_mut();
            if !selected.contains(&self.key) {
                let index = index.min(selected.len());
                selected.insert(index, self.key);
            }
        }
        true
    }
}
