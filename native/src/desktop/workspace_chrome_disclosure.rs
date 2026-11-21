//! 每行展开覆盖与 Tab 总开关同步；只保存瞬态几何意图，不改变端口显隐或连接图。
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

/// 同一宿主/字段可跨目录换代保留；Tab 边沿重置该节点所有局部覆盖，不能影响其它节点。
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

/// 复用属性栏的同步关闭入口；Live 宿主不是 State，不能把其 USERDATA 解引用为文档。
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

/// 左侧属性栏显示前同步收起两侧内容；只变呈现，不销毁参数事务或更改底部抽屉。
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
    // 同步移除 child/候选可见区，不能等 Application 空闲时才修正重叠。
    window::refresh(&ui)
}

fn names(endpoint: &desktop::link_catalog::Endpoint, side: Side) -> &[String] {
    match side {
        Side::Input => &endpoint.inputs,
        Side::Output => &endpoint.outputs,
    }
}

#[cfg(test)]
#[path = "workspace_chrome_disclosure_tests.rs"]
mod tests;
