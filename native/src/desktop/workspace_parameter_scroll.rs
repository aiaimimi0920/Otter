//! 参数滚动即时更新同一几何；thumb 捕获冻结比例，Tab 开关保留每 Unit 的有界滚动位置。
use super::*;
use windows::Win32::{Graphics::Gdi::ClientToScreen, UI::Input::KeyboardAndMouse::*};
pub(in crate::desktop) unsafe fn reveal_parameter(binding: &Binding, host: HWND, id: &str) {
    let ui = binding.borrow().upgrade();
    if let Some(ui) = ui {
        reveal(&ui, host, id);
    }
}
pub(in crate::desktop) fn close_parameter_rows(binding: &Binding, host: HWND) {
    let ui = binding.borrow().upgrade();
    if let Some(ui) = ui {
        if let Some(node) = ui.nodes.borrow().iter().find(|n| n.hwnd == host.0 as isize) {
            for index in 0..node.inputs {
                ui.disclosed.borrow_mut().insert(
                    Port {
                        node: node.endpoint,
                        side: Side::Input,
                        index,
                    },
                    false,
                );
            }
        }
    }
}
pub(in crate::desktop) unsafe fn focus_parameter(
    binding: &Binding,
    host: HWND,
    id: &str,
    key: usize,
) {
    let ui = binding.borrow().upgrade();
    let Some(ui) = ui else {
        return;
    };
    let Some(node) = ui
        .nodes
        .borrow()
        .iter()
        .find(|n| n.hwnd == host.0 as isize)
        .cloned()
    else {
        return;
    };
    let scene = ui.scene.borrow().clone();
    let Some(scene) = scene else {
        return;
    };
    let Some(index) = scene.catalog.endpoints[node.endpoint]
        .inputs
        .iter()
        .rposition(|name| name == id)
    else {
        return;
    };
    keyboard::restore(
        &ui,
        Port {
            node: node.endpoint,
            side: Side::Input,
            index,
        },
    );
    ui.keyboard.suppress(key);
}
#[derive(Clone, Copy)]
pub(super) struct Drag {
    hwnd: HWND,
    endpoint: usize,
    y: i32,
    offset: f64,
    max: f64,
    travel: i32,
}
fn set(ui: &Ui, endpoint: usize, offset: f64) {
    let Some(scene) = ui.scene.borrow().clone() else {
        return;
    };
    let Some(entry) = scene.catalog.endpoints.get(endpoint) else {
        return;
    };
    let mut offsets = ui.rail_offsets.borrow_mut();
    offsets.retain(|(id, _)| *id != entry.unit);
    if offsets.len() >= 512 {
        offsets.remove(0);
    }
    offsets.push((entry.unit.clone(), offset));
}
pub(super) unsafe fn reveal(ui: &Ui, host: HWND, id: &str) {
    let Some(node) = ui
        .nodes
        .borrow()
        .iter()
        .find(|n| n.hwnd == host.0 as isize)
        .cloned()
    else {
        return;
    };
    let Some(scene) = ui.scene.borrow().clone() else {
        return;
    };
    let Some(index) = scene.catalog.endpoints[node.endpoint]
        .inputs
        .iter()
        .rposition(|name| name == id)
    else {
        return;
    };
    let Some(rail) = parameter_rail::layout(ui, &node) else {
        return;
    };
    let Some(row) = rail.row(index) else {
        return;
    };
    let change = if row.y < rail.viewport.y {
        row.y - rail.viewport.y
    } else if row.y + row.height > rail.viewport.y + rail.viewport.height {
        row.y + row.height - rail.viewport.y - rail.viewport.height
    } else {
        return;
    };
    set(
        ui,
        node.endpoint,
        rail.scroll(f64::from(change) * 96.0 / f64::from(node.dpi)),
    );
    if let Err(error) = window::refresh(ui) {
        ui.fail(error);
    }
}
fn contains(f: neuro_mini_ui::Frame, x: i32, y: i32) -> bool {
    (f.x..f.x + f.width).contains(&x) && (f.y..f.y + f.height).contains(&y)
}
pub(super) unsafe fn message(hwnd: HWND, ui: &Ui, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    if ui.blocked() {
        return false;
    }
    let Some((_, endpoint, Side::Input)) = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(h, _, _)| *h == hwnd)
        .copied()
    else {
        return false;
    };
    let Some(node) = ui.node(endpoint) else {
        return false;
    };
    let Some(rail) = parameter_rail::layout(ui, &node) else {
        return false;
    };
    let mut p = POINT {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 >> 16) as u16 as i16 as i32,
    };
    if msg == WM_MOUSEWHEEL {
        if !contains(rail.viewport, p.x, p.y) {
            return false;
        }
        let delta = (wp.0 >> 16) as u16 as i16;
        set(ui, endpoint, rail.scroll(-f64::from(delta) * 48.0 / 120.0));
    } else {
        if !ClientToScreen(hwnd, &mut p).as_bool() {
            return false;
        }
        if msg == WM_LBUTTONDOWN
            && parameter_rail::active(ui, &node)
            && rail.track().is_some_and(|t| contains(t, p.x, p.y))
        {
            let thumb = rail.thumb().unwrap();
            if contains(thumb, p.x, p.y) {
                ui.rail_drag.set(Some(Drag {
                    hwnd,
                    endpoint,
                    y: p.y,
                    offset: rail.offset,
                    max: rail.max_scroll,
                    travel: rail.track().unwrap().height - thumb.height,
                }));
                SetCapture(hwnd);
                if GetCapture() != hwnd {
                    ui.rail_drag.set(None);
                }
            } else {
                set(ui, endpoint, rail.track_offset(p.y));
            }
        } else if let Some(drag) = ui
            .rail_drag
            .get()
            .filter(|d| d.hwnd == hwnd && d.endpoint == endpoint)
        {
            if msg == WM_MOUSEMOVE || msg == WM_LBUTTONUP {
                if drag.travel > 0 {
                    set(
                        ui,
                        endpoint,
                        (drag.offset + f64::from(p.y - drag.y) / f64::from(drag.travel) * drag.max)
                            .clamp(0.0, drag.max),
                    );
                }
                if msg == WM_LBUTTONUP {
                    ui.rail_drag.set(None);
                    if GetCapture() == hwnd {
                        let _ = ReleaseCapture();
                    }
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    if let Err(error) = window::refresh(ui) {
        ui.fail(error);
    }
    true
}
