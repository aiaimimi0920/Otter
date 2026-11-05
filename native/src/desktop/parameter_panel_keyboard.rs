//! 宿主裸 Tab 打开参数窗；窗内 Tab 导航，Enter 提交草稿，IME 和打开的列表先消费键盘。
use super::*;
use windows::Win32::UI::Shell::{
    DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass,
};
const EDIT_POLICY: usize = 0x4f545050;
fn toggles(msg: u32, key: usize, control: bool, shift: bool, alt: bool) -> bool {
    msg == WM_KEYDOWN && key == VK_TAB.0 as usize && !control && !shift && !alt
}
pub(super) unsafe fn host_key(msg: u32, key: usize) -> bool {
    toggles(
        msg,
        key,
        GetKeyState(VK_CONTROL.0 as i32) < 0,
        GetKeyState(VK_SHIFT.0 as i32) < 0,
        GetKeyState(VK_MENU.0 as i32) < 0,
    )
}
pub(super) unsafe fn enter(ui: &Ui) {
    let child = ui
        .bindings
        .borrow()
        .iter()
        .find(|b| b.enabled)
        .map(|b| b.hwnd);
    if let Some(child) = child {
        let _ = SetFocus(Some(child));
        layout::reveal(ui, child);
    }
}
pub(super) unsafe fn translate(ui: &Ui, msg: &MSG) -> bool {
    let panel = ui.hwnd.get();
    if ui.state().is_none()
        || ui.closed.get()
        || !IsWindow(Some(panel)).as_bool()
        || !(msg.hwnd == panel || IsChild(panel, msg.hwnd).as_bool())
    {
        return false;
    }
    if !ui.available() {
        if msg.message == WM_KEYDOWN
            && msg.wParam.0 == VK_ESCAPE.0 as usize
            && msg.lParam.0 & (1 << 30) == 0
        {
            ui.close();
        }
        return desktop::session_end::blocks_input(msg.message)
            || (WM_KEYFIRST..=WM_KEYLAST).contains(&msg.message);
    }
    if msg.message == WM_KEYDOWN {
        let focus = GetFocus();
        if composing(focus) {
            return false;
        }
        // 标准 COMBOBOX 已展开时，Esc/Enter 属于列表而非面板关闭或执行。
        let binding = ui
            .bindings
            .borrow()
            .iter()
            .find(|b| b.hwnd == focus)
            .cloned();
        if binding
            .as_ref()
            .is_some_and(|b| matches!(b.kind, controls::Kind::Options(..)))
            && SendMessageW(focus, CB_GETDROPPEDSTATE, None, None).0 != 0
            && [VK_ESCAPE.0 as usize, VK_RETURN.0 as usize].contains(&msg.wParam.0)
        {
            return false;
        }
        if msg.wParam.0 == VK_ESCAPE.0 as usize {
            if msg.lParam.0 & (1 << 30) == 0 {
                if let Some(controls::Binding {
                    kind: controls::Kind::Text { initial, .. },
                    ..
                }) = &binding
                {
                    if controls::text(focus).as_ref() != Some(initial) {
                        let _ = SetWindowTextW(focus, &HSTRING::from(initial));
                        ui.error("");
                        return true;
                    }
                }
                ui.close();
            }
            return true;
        }
        if msg.wParam.0 == VK_RETURN.0 as usize {
            if let Some(binding) = binding {
                if let controls::Kind::Text { spec, .. } = &binding.kind {
                    let control = GetKeyState(VK_CONTROL.0 as i32) < 0;
                    if !spec.multiline || control {
                        if msg.lParam.0 & (1 << 30) == 0 {
                            controls::commit(ui, &binding);
                        }
                        return true;
                    }
                    return false;
                }
            }
        }
    }
    let handled = IsDialogMessageW(panel, msg).as_bool();
    if handled {
        layout::reveal(ui, GetFocus());
    }
    handled
}
pub(super) unsafe fn composing(edit: HWND) -> bool {
    let mut state = 0;
    GetWindowSubclass(edit, Some(edit_policy), EDIT_POLICY, Some(&mut state)).as_bool()
        && state != 0
}
pub(super) unsafe fn install(edit: HWND) -> Result<()> {
    SetWindowSubclass(edit, Some(edit_policy), EDIT_POLICY, 0).ok()
}
unsafe extern "system" fn edit_policy(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    mut state: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(edit_policy), EDIT_POLICY);
    }
    if msg == WM_IME_STARTCOMPOSITION {
        state = 1;
    }
    if msg == WM_IME_ENDCOMPOSITION {
        state = 2;
    }
    if msg == WM_KEYUP || msg == WM_KEYDOWN && lp.0 & (1 << 30) == 0 {
        state &= 1;
    }
    if matches!(
        msg,
        WM_IME_STARTCOMPOSITION | WM_IME_ENDCOMPOSITION | WM_KEYDOWN | WM_KEYUP
    ) {
        let _ = SetWindowSubclass(hwnd, Some(edit_policy), EDIT_POLICY, state);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
