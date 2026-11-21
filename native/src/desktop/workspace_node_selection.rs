//! 节点选择是显式指针意图，不从前台焦点或扩展快捷键的最后目标推断。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
pub(in crate::desktop) unsafe fn observe(binding: &Binding, hwnd: HWND, msg: u32) {
    if !matches!(
        msg,
        WM_LBUTTONDOWN | WM_NCLBUTTONDOWN | WM_RBUTTONDOWN | WM_NCRBUTTONDOWN
    ) {
        return;
    }
    let Some(ui) = binding.borrow().upgrade() else {
        return;
    };
    if ui.blocked() {
        return;
    }
    let marker = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    let Some(node) = ui
        .nodes
        .borrow()
        .iter()
        .find(|n| n.hwnd == hwnd.0 as isize && n.marker == marker && n.interactive)
        .cloned()
    else {
        return;
    };
    let key = (node.hwnd, node.marker);
    let mut selected = ui.selected.borrow_mut();
    if GetKeyState(VK_CONTROL.0 as i32) >= 0 && !selected.contains(&key) {
        selected.clear();
    }
    if !selected.contains(&key) {
        selected.push(key);
    }
    drop(selected);
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
}
pub(super) fn reconcile(ui: &Ui) {
    let nodes = ui.nodes.borrow();
    ui.selected
        .borrow_mut()
        .retain(|&(hwnd, marker)| nodes.iter().any(|n| n.hwnd == hwnd && n.marker == marker));
}
