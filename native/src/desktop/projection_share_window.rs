//! 非模态 HWND 持有一份 Rc 至 NCDESTROY；回调不跨 UI 重入保留 RefCell 借用。
use super::*;
use windows::Win32::{
    Graphics::Gdi::*,
    UI::{
        Controls::*,
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled},
        Shell::*,
    },
};
#[cfg(test)]
#[path = "projection_share_window_tests.rs"]
mod tests;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
pub(super) unsafe fn create(model: &Rc<Model>) -> Result<()> {
    let template = layout::template();
    let hwnd = CreateDialogIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(model.owner),
        Some(procedure),
        LPARAM(Rc::as_ptr(model) as isize),
    )?;
    let mut affinity = 0;
    if GetWindowDisplayAffinity(model.owner, &mut affinity).is_ok() {
        let _ = SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(affinity));
    }
    place(model);
    let _ = ShowWindow(hwnd, SW_SHOW);
    Ok(())
}
pub(super) unsafe fn enable(hwnd: HWND, id: i32, enabled: bool) {
    if let Ok(control) = GetDlgItem(Some(hwnd), id) {
        if IsWindowEnabled(control).as_bool() != enabled {
            let _ = EnableWindow(control, enabled);
        }
    }
}
pub(super) unsafe fn place(m: &Model) {
    let Some(hwnd) = m.hwnd.get() else {
        return;
    };
    let mut rect = RECT::default();
    if GetWindowRect(m.owner, &mut rect).is_err() {
        return;
    }
    let dpi = GetDpiForWindow(m.owner).max(96);
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(
        MonitorFromWindow(m.owner, MONITOR_DEFAULTTONEAREST),
        &mut monitor,
    )
    .as_bool()
    {
        return;
    }
    let work = monitor.rcWork;
    let width = (layout::width(m) * dpi as i32 / 96).min(work.right - work.left);
    let height = (layout::height(m) * dpi as i32 / 96).min(work.bottom - work.top);
    let x = (rect.left + (rect.right - rect.left - width) / 2).clamp(work.left, work.right - width);
    let y =
        (rect.top + (rect.bottom - rect.top - height) / 2).clamp(work.top, work.bottom - height);
    let mut old = RECT::default();
    if GetWindowRect(hwnd, &mut old).is_ok()
        && old
            == (RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            })
    {
        return;
    }
    let _ = SetWindowPos(
        hwnd,
        None,
        x,
        y,
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    layout::arrange(hwnd, m, dpi);
}
pub(super) unsafe fn output(hwnd: HWND, m: &Model, visible: bool) {
    if !visible
        && m.code.borrow().is_none()
        && m.account_envelope.borrow().is_none()
        && m.link.borrow().is_empty()
    {
        return; // 空输出不反复 invalidate，否则空闲 WM_PAINT 会自驱动消息循环。
    }
    if !visible {
        m.code.borrow_mut().take();
        m.account_envelope.borrow_mut().take();
        m.envelope.borrow_mut().take();
        m.link.borrow_mut().clear();
    }
    if !m.receive {
        let text = m.link.borrow().clone();
        input(hwnd, if visible { &text } else { "" });
    }
    enable(hwnd, 1, visible);
    place(m);
    layout::arrange(hwnd, m, GetDpiForWindow(hwnd));
    let _ = InvalidateRect(Some(hwnd), None, true);
}
pub(super) unsafe fn input(hwnd: HWND, value: &str) {
    let mut buffer = vec![0u16; MAX_TEXT + 1];
    let length = GetDlgItemTextW(hwnd, 101, &mut buffer) as usize;
    if buffer[..length].iter().copied().ne(value.encode_utf16()) {
        let _ = SetDlgItemTextW(hwnd, 101, &HSTRING::from(value));
    }
}
pub(super) unsafe fn label(hwnd: HWND, id: i32, value: &str) {
    let mut old = [0u16; 64];
    let length = GetDlgItemTextW(hwnd, id, &mut old) as usize;
    if old[..length].iter().copied().ne(value.encode_utf16()) {
        let _ = SetDlgItemTextW(hwnd, id, &HSTRING::from(value));
    }
}
unsafe fn copy(m: &Model) -> Result<()> {
    let value = m.link.borrow().clone();
    let hwnd = m
        .hwnd
        .get()
        .ok_or_else(|| Error::new(E_ABORT, "Share closed"))?;
    if value.is_empty() || !m.output_current() {
        return Err(Error::new(E_ABORT, "Invitation unavailable"));
    }
    crate::desktop::clipboard::copy_text_checked(hwnd, &value, || {
        m.output_current() && m.hwnd.get() == Some(hwnd)
    })?;
    let _ = SetDlgItemTextW(hwnd, 1, &HSTRING::from("已复制"));
    if let Ok(edit) = GetDlgItem(Some(hwnd), 101) {
        SendMessageW(edit, 0x00b1, Some(WPARAM(0)), Some(LPARAM(-1)));
    }
    Ok(())
}
unsafe extern "system" fn edit_proc(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    // 默认 EDIT 处理也可重入父窗口并触发销毁；整个回调持有模型。
    let ptr = data as *const Model;
    Rc::increment_strong_count(ptr);
    let m = Rc::from_raw(ptr);
    if message == WM_COPY && !m.receive && !m.output_current() {
        return LRESULT(0);
    }
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(edit_proc), id);
    }
    result
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        let ptr = lp.0 as *const Model;
        Rc::increment_strong_count(ptr);
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        let m = &*ptr;
        m.hwnd.set(Some(hwnd));
        for id in [1, 2, 103, 104, 105, 108, 109, 110] {
            if let Ok(button) = GetDlgItem(Some(hwnd), id) {
                let _ = crate::desktop::dialog_controls::button(button);
            }
        }
        if let Ok(edit) = GetDlgItem(Some(hwnd), 101) {
            let _ = crate::desktop::dialog_controls::edit(edit);
            SendMessageW(edit, 0x00c5, Some(WPARAM(MAX_TEXT)), Some(LPARAM(0)));
            SendMessageW(
                edit,
                0x00cf,
                Some(WPARAM(usize::from(!m.receive))),
                Some(LPARAM(0)),
            );
            let _ = SetWindowSubclass(edit, Some(edit_proc), 1, ptr as usize);
        }
        if !m.receive {
            let _ = SetDlgItemTextW(hwnd, 1, &HSTRING::from("复制链接"));
        }
        enable(hwnd, 1, false);
        layout::arrange(hwnd, m, GetDpiForWindow(hwnd));
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Model;
    if ptr.is_null() {
        return 0;
    }
    // 文件选择、剪贴板及同步 HWND 消息均可能销毁当前窗口。
    Rc::increment_strong_count(ptr);
    let retained = Rc::from_raw(ptr);
    let m = &*retained;
    match message {
        WM_CLOSE => {
            m.create_pending.set(false);
            m.cancel_decode();
            m.cancel.store(true, Ordering::Release);
            let _ = DestroyWindow(hwnd);
            1
        }
        WM_NCDESTROY => {
            m.create_pending.set(false);
            m.cancel_decode();
            m.cancel.store(true, Ordering::Release);
            m.hwnd.set(None);
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), 0);
            drop(Rc::from_raw(ptr));
            0
        }
        WM_COMMAND => {
            let id = wp.0 & 0xffff;
            let notification = (wp.0 >> 16) as u32;
            let result = match id {
                108 if !m.receive || m.public.get() => {
                    m.create_pending.set(false);
                    if m.public.get() {
                        m.stop_account()
                    } else {
                        m.stop_lan()
                    }
                }
                109 | 110 => m.switch_scope(id == 110),
                2 => {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    Ok(())
                }
                1 if m.receive => m.submit(),
                1 => copy(m),
                101 if notification == EN_CHANGE
                    && m.receive
                    && !m.qr.get()
                    && !m.account_import.get() =>
                {
                    let mut buffer = vec![0u16; MAX_TEXT + 1];
                    let count = GetDlgItemTextW(hwnd, 101, &mut buffer) as usize;
                    m.parsed(&String::from_utf16_lossy(&buffer[..count]));
                    Ok(())
                }
                104 | 105 => {
                    m.switch_tab(id == 104);
                    Ok(())
                }
                103 if m.receive && !m.account_import.get() => choose(&retained),
                _ => Ok(()),
            };
            if let Err(error) = result {
                m.set_status(&error.to_string());
            }
            1
        }
        WM_DRAWITEM => {
            draw(m, &*(lp.0 as *const DRAWITEMSTRUCT));
            1
        }
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
            crate::desktop::dialog_controls::background(message, wp, lp).unwrap()
        }
        _ => 0,
    }
}
unsafe fn choose(m: &Rc<Model>) -> Result<()> {
    use windows::Win32::UI::Controls::Dialogs::*;
    let _guard = crate::desktop::file_dialog::Guard::begin(&m.file_dialog_active)
        .ok_or_else(|| Error::new(E_ABORT, "File dialog already active"))?;
    let view = m
        .view
        .upgrade()
        .ok_or_else(|| Error::new(E_ABORT, "Share closed"))?;
    if view.borrow().share_decode.is_some() {
        return Err(Error::new(E_ABORT, "上一张图片仍在结束识别，请稍后重试"));
    }
    let epoch = m.input_epoch.get();
    let hwnd = m
        .hwnd
        .get()
        .ok_or_else(|| Error::new(E_ABORT, "Share closed"))?;
    let mut path = [0u16; 32768];
    let filter: Vec<u16> = "PNG 图片\0*.png\0\0".encode_utf16().collect();
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: windows::core::PWSTR(path.as_mut_ptr()),
        nMaxFile: path.len() as u32,
        lpstrFilter: windows::core::PCWSTR(filter.as_ptr()),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER,
        ..Default::default()
    };
    if !GetOpenFileNameW(&mut options).as_bool() {
        return if CommDlgExtendedError() == COMMON_DLG_ERRORS(0) {
            Ok(())
        } else {
            Err(Error::new(E_FAIL, "PNG file dialog failed"))
        };
    }
    if m.cancel.load(Ordering::Acquire)
        || m.hwnd.get() != Some(hwnd)
        || !m.qr.get()
        || m.input_epoch.get() != epoch
        || view.borrow().origin.as_deref() != Some(&m.origin)
    {
        return Ok(());
    }
    let len = path.iter().position(|c| *c == 0).unwrap_or(path.len());
    let path = std::path::PathBuf::from(String::from_utf16_lossy(&path[..len]));
    if view.borrow().share_decode.is_some() {
        return Err(Error::new(E_ABORT, "上一张图片仍在结束识别，请稍后重试"));
    }
    m.cancel_decode();
    m.account_envelope.borrow_mut().take();
    m.envelope.borrow_mut().take();
    enable(hwnd, 1, false);
    view.borrow_mut().share_decode = Some(qr::Decode::start(path, m)?);
    m.set_status("识别中…");
    Ok(())
}
unsafe fn draw(m: &Model, item: &DRAWITEMSTRUCT) {
    let dc = item.hDC;
    let rect = item.rcItem;
    if item.CtlID == 106 {
        let white = HBRUSH(GetStockObject(WHITE_BRUSH).0);
        let black = HBRUSH(GetStockObject(BLACK_BRUSH).0);
        FillRect(dc, &rect, white);
        if let Some(code) = m.code.borrow().as_ref() {
            let cell = ((rect.right - rect.left).min(rect.bottom - rect.top)
                / (code.size as i32 + 8))
                .max(1);
            let side = cell * code.size as i32;
            let left = (rect.left + rect.right - side) / 2;
            let top = (rect.top + rect.bottom - side) / 2;
            for (i, color) in code.colors.iter().enumerate() {
                if *color == qrcode::Color::Dark {
                    let x = left + (i % code.size) as i32 * cell;
                    let y = top + (i / code.size) as i32 * cell;
                    FillRect(
                        dc,
                        &RECT {
                            left: x,
                            top: y,
                            right: x + cell,
                            bottom: y + cell,
                        },
                        black,
                    );
                }
            }
        }
        return;
    }
    let active = (item.CtlID == 104 && m.qr.get())
        || (item.CtlID == 105 && !m.qr.get())
        || (item.CtlID == 109 && !m.public.get())
        || (item.CtlID == 110 && m.public.get());
    crate::desktop::dialog_controls::draw_button(item, active);
}
