//! Wiring between attached Tab parameters and the sole transaction owner; DocumentWindow still destroys windows before the host.
use super::*;
use desktop::document_window::parameter_panel::Panel;
use neuro_mini_ui::{Frame, RailLayout};
use windows::Win32::Graphics::Gdi::MONITORINFO;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "workspace_chrome_capsule_tests.rs"]
mod disclosure_windows;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "workspace_parameter_rail_tests.rs"]
mod tests;
pub(super) struct Focus {
    port: Port,
    catalog: Rc<Catalog>,
    host: isize,
    marker: isize,
}
pub(super) fn active(ui: &Ui, node: &Node) -> bool {
    ui.scene.borrow().as_ref().is_some_and(|scene| {
        let entry = &scene.catalog.endpoints[node.endpoint];
        (entry.parameter_start..entry.inputs.len()).any(|index| {
            disclosure::expanded(
                ui,
                Port {
                    node: node.endpoint,
                    side: Side::Input,
                    index,
                },
                node,
            )
        })
    })
}
pub(super) fn layout(ui: &Ui, node: &Node) -> Option<RailLayout> {
    let scene = ui.scene.borrow().clone()?;
    let entry = scene.catalog.endpoints.get(node.endpoint)?;
    if node.minified || entry.parameter_start == entry.inputs.len() {
        return None;
    }
    let scale = |n: i32| (n * node.dpi.clamp(96, 768) as i32 + 48) / 96;
    let height = ui
        .rail_heights
        .borrow()
        .iter()
        .find(|(host, dpi, _)| *host == node.hwnd && *dpi == node.dpi)
        .map_or(scale(360), |(_, _, height)| *height);
    let offset = ui
        .rail_offsets
        .borrow()
        .iter()
        .find(|(unit, _)| *unit == entry.unit)
        .map_or(0.0, |(_, y)| *y);
    Some(RailLayout::new(
        Frame {
            x: node.rect.left - scale(242) - desktop::pin_border_style::width(node.dpi) as i32,
            y: node.rect.top + scale(22),
            width: scale(256),
            height,
        },
        node.dpi,
        entry.parameter_start,
        entry.inputs.len() - entry.parameter_start,
        offset,
    ))
}
pub(super) unsafe fn request(ui: &Ui, port: Port, enter: bool) -> bool {
    let Some(node) = ui
        .node(port.node)
        .filter(|node| node.interactive && !node.minified)
    else {
        return false;
    };
    if ui.blocked() || !node.ports(ui.mode.get(), node.hide_collapsed_endpoints) {
        return false;
    }
    let Some(scene) = ui.scene.borrow().clone() else {
        return false;
    };
    let Some(entry) = scene.catalog.endpoints.get(port.node) else {
        return false;
    };
    if port.side != Side::Input || port.index < entry.parameter_start {
        return false;
    }
    if port.index >= entry.inputs.len()
        || !IsWindowVisible(HWND(node.hwnd as *mut _)).as_bool()
        || GetWindowLongPtrW(HWND(node.hwnd as *mut _), GWLP_USERDATA) != node.marker
        || PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0)).is_err()
    {
        return false;
    }
    disclosure::close_settings(&node);
    ui.disclosed.borrow_mut().insert(port, true);
    // F2/programmatic expansion lacks post-mouse-release refresh; update parent-strip clipping and children once within the original synchronous segment.
    ui.rail_layout_dirty.set(true);
    if enter {
        *ui.rail_focus.borrow_mut() = Some(Focus {
            port,
            catalog: scene.catalog.clone(),
            host: node.hwnd,
            marker: node.marker,
        });
    }
    true
}
pub(super) unsafe fn context(ui: &Ui, hwnd: HWND, lp: LPARAM) -> bool {
    let Some((_, endpoint, Side::Input)) = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(h, _, _)| *h == hwnd)
        .copied()
    else {
        return false;
    };
    let Some(scene) = ui.scene.borrow().clone() else {
        return false;
    };
    let Some((_, panel)) = ui
        .rails
        .borrow()
        .iter()
        .find(|(n, _)| *n == endpoint)
        .cloned()
    else {
        return false;
    };
    let point = Point {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 >> 16) as u16 as i16 as i32,
    };
    let entry = &scene.catalog.endpoints[endpoint];
    let Some(node) = ui.node(endpoint) else {
        return false;
    };
    let geometry = port_visual::SideGeometry::new(ui, &node, Side::Input);
    for (index, id) in entry.inputs.iter().enumerate().skip(entry.parameter_start) {
        let Some(layout) = geometry.layout(index, true) else {
            continue;
        };
        if Rect::from(geometry.rect(index, &layout)).contains(point) {
            panel.context(id, point.x >= layout.body.x + (104 * node.dpi / 96) as i32);
            return true;
        }
    }
    false
}
pub(super) unsafe fn sync(app: &mut Application) -> Result<()> {
    let ui = &app.chrome.ui;
    let Some(scene) = ui.scene.borrow().clone() else {
        return Ok(());
    };
    let mut heights = Vec::new();
    let mut bindings = Vec::new();
    let mut open = app
        .documents
        .iter()
        .filter(|d| d.parameters.is_some())
        .count();
    for doc in &mut app.documents {
        if doc.state.local_art_id.is_some() {
            continue;
        }
        let Some(node) = ui
            .nodes
            .borrow()
            .iter()
            .find(|n| n.hwnd == doc.window.0 .0 as isize)
            .cloned()
        else {
            continue;
        };
        let entry = &scene.catalog.endpoints[node.endpoint];
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        windows::Win32::Graphics::Gdi::GetMonitorInfoW(
            windows::Win32::Graphics::Gdi::MonitorFromWindow(
                doc.window.0,
                windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
            ),
            &mut info,
        )
        .ok()?;
        let height = (info.rcWork.bottom - info.rcWork.top - (300 * node.dpi / 96) as i32).max(1);
        heights.push((node.hwnd, node.dpi, height));
        let needed = node.interactive
            && !node.minified
            && node.ports(ui.mode.get(), node.hide_collapsed_endpoints)
            && (candidate_auxiliary::layout(ui, &node).is_some()
                || (entry.parameter_start..entry.inputs.len()).any(|index| {
                    disclosure::expanded(
                        ui,
                        Port {
                            node: node.endpoint,
                            side: Side::Input,
                            index,
                        },
                        &node,
                    )
                }));
        if !needed {
            if doc
                .parameters
                .as_ref()
                .is_some_and(|p| p.inline_binding().is_some())
            {
                if doc.parameters.as_ref().unwrap().exit_idle() {
                    doc.parameters.take();
                    open -= 1;
                } else {
                    let _ = ShowWindow(doc.parameters.as_ref().unwrap().handle(), SW_HIDE);
                    bindings.push((
                        node.endpoint,
                        doc.parameters.as_ref().unwrap().inline_binding().unwrap(),
                    ));
                }
            }
            continue;
        }
        let parent = ui
            .input_windows
            .borrow()
            .iter()
            .find(|(_, n, s)| *n == node.endpoint && *s == Side::Input)
            .map(|(h, _, _)| *h)
            .or_else(|| {
                ui.input_windows
                    .borrow()
                    .iter()
                    .find(|(_, n, s)| *n == node.endpoint && *s == Side::Output)
                    .map(|(h, _, _)| *h)
            });
        let Some(parent) = parent else {
            continue;
        };
        if doc.parameters.is_none() {
            if open >= 4 {
                doc.state.parameters_ui.expanded.set(false);
                desktop::extension_notices::show(
                    doc.window.0,
                    &doc.state,
                    None,
                    crate::ui_text::text("parameter-title"),
                    crate::ui_text::text("workspace-parameter-limit"),
                )?;
                continue;
            }
            doc.parameters = match Panel::inline(doc.window.0, &doc.state, parent) {
                Ok(panel) => Some(panel),
                Err(error) => {
                    doc.state.parameters_ui.expanded.set(false);
                    return Err(error);
                }
            };
            open += 1;
        }
        if let Some(binding) = doc.parameters.as_ref().and_then(Panel::inline_binding) {
            bindings.push((node.endpoint, binding));
        }
    }
    let changed = *ui.rail_heights.borrow() != heights;
    *ui.rail_heights.borrow_mut() = heights;
    *ui.rails.borrow_mut() = bindings;
    let requested_layout = ui.rail_layout_dirty.replace(false);
    if changed || requested_layout {
        window::refresh(ui)?;
    }
    place(ui)?;
    let focus = ui.rail_focus.borrow_mut().take();
    if let Some(focus) = focus.filter(|focus| {
        Rc::ptr_eq(&focus.catalog, &scene.catalog)
            && ui.node(focus.port.node).is_some_and(|node| {
                node.hwnd == focus.host
                    && node.marker == focus.marker
                    && disclosure::expanded(ui, focus.port, &node)
            })
    }) {
        let port = focus.port;
        if let Some(entry) = scene.catalog.endpoints.get(port.node) {
            if let Some(id) = entry.inputs.get(port.index) {
                let panel = ui
                    .rails
                    .borrow()
                    .iter()
                    .find(|(n, _)| *n == port.node)
                    .map(|(_, p)| p.clone());
                if let Some(panel) = panel {
                    panel.focus(id);
                }
            }
        }
    }
    Ok(())
}
pub(super) unsafe fn place(ui: &Ui) -> Result<()> {
    let bindings = ui.rails.borrow().clone();
    for (endpoint, panel) in bindings {
        candidate_auxiliary::place(ui, endpoint, &panel)?;
        let Some(node) = ui.node(endpoint) else {
            continue;
        };
        let Some(scene) = ui.scene.borrow().clone() else {
            continue;
        };
        let Some((parent, _, _)) = ui
            .input_windows
            .borrow()
            .iter()
            .find(|(_, n, s)| *n == endpoint && *s == Side::Input)
            .copied()
        else {
            continue;
        };
        let mut bounds = RECT::default();
        GetWindowRect(parent, &mut bounds)?;
        let geometry = port_visual::SideGeometry::new(ui, &node, Side::Input);
        let mut rows = Vec::new();
        for (index, id) in scene.catalog.endpoints[endpoint]
            .inputs
            .iter()
            .enumerate()
            .skip(scene.catalog.endpoints[endpoint].parameter_start)
        {
            if !disclosure::expanded(ui, geometry.port(index), &node) {
                continue;
            }
            let Some(layout) = geometry.layout(index, false) else {
                continue;
            };
            let f = layout.body;
            let row = RECT {
                left: f.x,
                top: f.y,
                right: f.x + f.width,
                bottom: f.y + f.height,
            };
            rows.push((id.clone(), row, geometry.clip(index).unwrap_or(row)));
        }
        panel.place(bounds, rows)?;
    }
    Ok(())
}
