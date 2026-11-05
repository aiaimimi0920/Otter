//! 控件通知只排有界意图；提交/模态及快照刷新在 Application 调度段执行。
use super::*;
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    std::panic::catch_unwind(|| dispatch(hwnd, msg, wp, lp)).unwrap_or(LRESULT(0))
}
unsafe fn dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Ui;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let ui = &*pointer;
    if msg == WM_NCDESTROY {
        ui.closed.set(true);
        ui.pending.borrow_mut().clear();
        ui.capture_windows
            .borrow_mut()
            .retain(|window| *window != hwnd);
        if let Some(state) = ui.state() {
            if state.parameters_ui.hwnd.get() == hwnd {
                state.parameters_ui.hwnd.set(HWND::default());
                state.parameters_ui.identity.set(0);
            }
        }
        desktop::tool_panel::font::release(hwnd);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    if ui.replacing.get() && matches!(msg, WM_COMMAND | WM_HSCROLL) {
        return LRESULT(0);
    }
    if matches!(msg, WM_COMMAND | WM_HSCROLL) && !ui.available() {
        return LRESULT(0);
    }
    if layout::scroll(ui, msg, wp, lp) {
        return LRESULT(0);
    }
    match msg {
        WM_COMMAND => {
            command(ui, wp, lp);
            LRESULT(0)
        }
        WM_HSCROLL => {
            let bindings = ui.bindings.borrow().clone();
            for binding in bindings {
                if let controls::Kind::Slider(slider, edit, snapshot, index) = binding.kind {
                    if let Some(text) = slider.changed(lp) {
                        let _ = SetWindowTextW(edit, &HSTRING::from(&text));
                        if wp.0 & 0xffff != TB_THUMBTRACK as usize {
                            ui.queue(Intent::Choice(snapshot, index, Some(text)));
                        }
                        break;
                    }
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            ui.close();
            LRESULT(0)
        }
        WM_SIZE => {
            layout::place(ui);
            LRESULT(0)
        }
        WM_DPICHANGED | WM_DPICHANGED_AFTERPARENT => {
            let _ = desktop::tool_panel::font::update(hwnd, GetDpiForWindow(hwnd));
            if msg == WM_DPICHANGED && ui.field.is_none() {
                let rect = *(lp.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
            let placements = ui.placements.borrow().clone();
            for (child, _) in placements {
                SendMessageW(
                    child,
                    WM_SETFONT,
                    Some(WPARAM(desktop::tool_panel::font::object(hwnd).0 as usize)),
                    Some(LPARAM(1)),
                );
            }
            let bindings = ui.bindings.borrow().clone();
            for binding in bindings {
                if matches!(binding.kind, controls::Kind::Options(..)) {
                    if let Err(error) = theme::options::resize(binding.hwnd) {
                        ui.error(&error.to_string());
                    }
                }
            }
            layout::place(ui);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            if lp.0 != 0 {
                let info = &mut *(lp.0 as *mut MINMAXINFO);
                info.ptMinTrackSize = POINT { x: 300, y: 220 };
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            theme::fill(HDC(wp.0 as *mut _), &rect, theme::PANEL);
            LRESULT(1)
        }
        WM_DRAWITEM if lp.0 != 0 => {
            let item = &*(lp.0 as *const DRAWITEMSTRUCT);
            if item.CtlType == ODT_COMBOBOX && GetParent(item.hwndItem).ok() == Some(hwnd) {
                theme::options::draw(item);
            } else if !controls::candidates::draw(ui, item) {
                theme::button(item);
            }
            LRESULT(1)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let error = lp.0 == ui.status.get().0 as isize && !ui.message.borrow().is_empty();
            theme::colors(msg, wp, lp, error)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
unsafe fn command(ui: &Ui, wp: WPARAM, lp: LPARAM) {
    let binding = ui
        .bindings
        .borrow()
        .iter()
        .find(|b| b.hwnd.0 as isize == lp.0)
        .cloned();
    let Some(binding) = binding.filter(|binding| binding.enabled) else {
        return;
    };
    let code = (wp.0 >> 16) as u32;
    match &binding.kind {
        controls::Kind::Text { .. } if code == EN_KILLFOCUS => controls::commit(ui, &binding),
        controls::Kind::Options(snapshot, indices) if code == CBN_SELCHANGE => {
            let selected = SendMessageW(binding.hwnd, CB_GETCURSEL, None, None).0;
            if let Ok(selected) = usize::try_from(selected) {
                if let Some(index) = indices.get(selected) {
                    ui.queue(Intent::Choice(snapshot.clone(), *index, None));
                }
            }
        }
        controls::Kind::Choice(snapshot, index) if code == BN_CLICKED => {
            ui.queue(Intent::Choice(snapshot.clone(), *index, None))
        }
        controls::Kind::Step(edit, spec, up) if code == BN_CLICKED => {
            if let Some(text) = controls::text(*edit).and_then(|text| spec.stepped(&text, *up)) {
                let _ = SetWindowTextW(*edit, &HSTRING::from(&text));
                let binding = ui
                    .bindings
                    .borrow()
                    .iter()
                    .find(|b| b.hwnd == *edit)
                    .cloned();
                if let Some(binding) = binding {
                    controls::commit(ui, &binding);
                }
            } else {
                ui.error("参数值无效");
            }
        }
        controls::Kind::Setting(stamp, index) if code == BN_CLICKED => {
            ui.queue(Intent::Setting(stamp.clone(), *index))
        }
        controls::Kind::Run(stamp) if code == BN_CLICKED => ui.queue(Intent::Run(stamp.clone())),
        controls::Kind::Stop(stamp) if code == BN_CLICKED => ui.queue(Intent::Stop(stamp.clone())),
        controls::Kind::Relay(stamp) if code == BN_CLICKED => {
            ui.queue(Intent::Relay(stamp.clone()))
        }
        controls::Kind::Ports(stamp) if code == BN_CLICKED => {
            ui.queue(Intent::Ports(stamp.clone()))
        }
        controls::Kind::Sticker(stamp, action) if code == BN_CLICKED => {
            ui.queue(Intent::Sticker(stamp.clone(), *action))
        }
        controls::Kind::Candidate(choice) if code == BN_CLICKED => {
            ui.queue(Intent::Candidate(choice.clone()))
        }
        controls::Kind::Page(page) if code == BN_CLICKED => ui.queue(Intent::Page(*page)),
        controls::Kind::Reload if code == BN_CLICKED => ui.queue(Intent::Reload),
        _ => {}
    }
}
