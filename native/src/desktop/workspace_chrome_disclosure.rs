//! Per-row expansion overrides synchronize with the master Tab toggle; store only transient geometry intents without port-visibility or graph changes.
use super::*;

pub(super) fn expanded(ui: &Ui, port: Port, node: &Node) -> bool {
    !node.minified
        && ui
            .disclosed
            .borrow()
            .get(&port)
            .copied()
            .unwrap_or(node.expanded)
}

pub(super) unsafe fn toggle(ui: &Ui, port: Port) -> bool {
    let Some(node) = ui.node(port.node).filter(|node| !node.minified) else {
        return false;
    };
    let next = !expanded(ui, port, &node);
    if next {
        close_settings(&node);
    }
    ui.disclosed.borrow_mut().insert(port, next);
    next
}

/// The same host/field may survive catalog generations; Tab edges reset all local overrides for that node, not others.
pub(super) unsafe fn sync(ui: &Ui, next: &Rc<Catalog>, nodes: &[Node]) {
    for node in nodes.iter().filter(|node| node.expanded) {
        if ui
            .node(node.endpoint)
            .is_none_or(|old| old.hwnd != node.hwnd || old.marker != node.marker || !old.expanded)
        {
            close_settings(node);
        }
    }
    if ui.disclosed.borrow().is_empty() {
        return;
    }
    if ui.scene.borrow().as_ref().is_some_and(|previous| {
        Rc::ptr_eq(&previous.catalog, next)
            && ui.nodes.borrow().len() == nodes.len()
            && ui.nodes.borrow().iter().zip(nodes).all(|(old, node)| {
                node.endpoint == old.endpoint
                    && node.hwnd == old.hwnd
                    && node.marker == old.marker
                    && node.expanded == old.expanded
                    && node.minified == old.minified
            })
    }) {
        return;
    }
    let mut restored = std::collections::HashMap::new();
    if let Some(previous) = ui.scene.borrow().as_ref() {
        for (&port, &expanded) in ui.disclosed.borrow().iter() {
            let Some(old) = previous.catalog.endpoints.get(port.node) else {
                continue;
            };
            let Some(name) = names(old, port.side).get(port.index) else {
                continue;
            };
            let Some(index) = next
                .endpoints
                .iter()
                .position(|entry| entry.unit == old.unit)
            else {
                continue;
            };
            if !ui.node(port.node).is_some_and(|old_node| {
                nodes.iter().any(|node| {
                    node.endpoint == index
                        && node.hwnd == old_node.hwnd
                        && node.marker == old_node.marker
                        && node.expanded == old_node.expanded
                        && node.minified == old_node.minified
                })
            }) {
                continue;
            }
            if let Some(position) = names(&next.endpoints[index], port.side)
                .iter()
                .position(|candidate| candidate == name)
            {
                restored.insert(
                    Port {
                        node: index,
                        side: port.side,
                        index: position,
                    },
                    expanded,
                );
            }
        }
    }
    *ui.disclosed.borrow_mut() = restored;
}

/// Reuse property-strip synchronous closure; Live hosts are not State, so never dereference their USERDATA as documents.
pub(super) unsafe fn close_settings(node: &Node) {
    let host = HWND(node.hwnd as *mut _);
    if GetWindowLongPtrW(host, GWLP_USERDATA) == node.marker
        && GetClassLongPtrW(host, GCLP_WNDPROC)
            == desktop::procedure::window_proc as *const () as usize
    {
        SendMessageW(
            host,
            desktop::tool_panel::EXECUTE,
            Some(WPARAM(0)),
            Some(LPARAM(neuro_mini_ui::PANEL_CLOSE)),
        );
    }
}

/// Collapse both sides synchronously before showing left properties; presentation only, without destroying parameter transactions or changing bottom drawers.
pub(in crate::desktop) unsafe fn collapse_for_settings(state: &State, host: HWND) -> Result<()> {
    state.parameters_ui.expanded.set(false);
    state.parameters_ui.toggle.set(false);
    let Some(ui) = state.chrome.borrow().upgrade() else {
        return Ok(());
    };
    let endpoint = {
        let mut nodes = ui.nodes.borrow_mut();
        let Some(node) = nodes
            .iter_mut()
            .find(|n| n.hwnd == host.0 as isize && n.marker == state as *const State as isize)
        else {
            return Ok(());
        };
        node.expanded = false;
        node.endpoint
    };
    ui.disclosed
        .borrow_mut()
        .retain(|port, _| port.node != endpoint);
    // Synchronously remove child/candidate visible regions; do not await Application idle to fix overlap.
    window::refresh(&ui)
}

fn names(endpoint: &desktop::link_catalog::Endpoint, side: Side) -> &[String] {
    match side {
        Side::Input => &endpoint.inputs,
        Side::Output => &endpoint.outputs,
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "workspace_chrome_disclosure_tests.rs"]
mod tests;
