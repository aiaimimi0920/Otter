//! Inline Tab visibility reuses the original single-slot appearance transaction; no modal dialog or link/execution-configuration changes.
use super::*;
pub(super) struct Toggle {
    catalog: Rc<Catalog>,
    port: Port,
}
pub(in super::super) unsafe fn request(ui: &Ui, port: Port) {
    if ui.blocked() || ui.port_settings.borrow().is_some() || ui.pending.borrow().is_some() {
        return;
    }
    let Some(node) = ui.node(port.node).filter(|n| n.interactive && !n.minified) else {
        return;
    };
    if !port_visual::layout(ui, port).is_some_and(|l| l.expanded) {
        return;
    }
    let Some(scene) = ui.scene.borrow().clone() else {
        return;
    };
    let Some(endpoint) = scene.catalog.endpoints.get(port.node) else {
        return;
    };
    let hwnd = HWND(node.hwnd as *mut _);
    if !IsWindowVisible(hwnd).as_bool() || GetWindowLongPtrW(hwnd, GWLP_USERDATA) != node.marker {
        return;
    }
    if PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0)).is_err() {
        return;
    }
    *ui.port_settings.borrow_mut() = Some(Open {
        unit: endpoint.unit.clone(),
        hwnd,
        marker: node.marker,
        toggle: Some(Toggle {
            catalog: scene.catalog.clone(),
            port,
        }),
    });
}
pub(super) unsafe fn commit(app: &mut Application, request: Open) {
    let Some(toggle) = request.toggle.as_ref() else {
        return;
    };
    if !app.port_settings_ready()
        || !app.port_settings_current(&request)
        || !toggle
            .catalog
            .matches(&app.documents, &app.live_sessions, &app.workspace)
        || !app
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .is_some_and(|s| Rc::ptr_eq(&s.catalog, &toggle.catalog))
        || !port_visual::layout(&app.chrome.ui, toggle.port).is_some_and(|l| l.expanded)
    {
        return;
    }
    let Some(endpoint) = toggle.catalog.endpoints.get(toggle.port.node) else {
        return;
    };
    let (names, mask) = match toggle.port.side {
        Side::Input => (&endpoint.inputs, &endpoint.input_visibility),
        Side::Output => (&endpoint.outputs, &endpoint.output_visibility),
    };
    let Some(name) = names.get(toggle.port.index) else {
        return;
    };
    let current = mask.get(toggle.port.index).copied().unwrap_or(true);
    let result = app
        .workspace
        .port_visibility(&request.unit)
        .edited(name, Some(!current))
        .and_then(|next| app.workspace.set_port_visibility(&request.unit, next));
    if let Err(error) = result {
        eprintln!("native-port-visibility: {error:?}");
    }
}
