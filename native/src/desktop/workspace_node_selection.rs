//! Node selection is explicit pointer intent, not inferred from foreground focus or the last extension-shortcut target.
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
impl Owner {
    #[cfg(test)]
    pub(in crate::desktop::application) unsafe fn select_for_test(&self, hwnd: HWND) {
        let key = (hwnd.0 as isize, GetWindowLongPtrW(hwnd, GWLP_USERDATA));
        let mut selected = self.ui.selected.borrow_mut();
        if !selected.contains(&key) {
            selected.push(key);
        }
    }
    pub(in crate::desktop::application) unsafe fn selected_units(
        &self,
    ) -> Option<Vec<crate::workspace_id::UnitId>> {
        let scene = self.ui.scene.borrow();
        let nodes = self.ui.nodes.borrow();
        let scene = scene.as_ref()?;
        self.ui
            .selected
            .borrow()
            .iter()
            .map(|&(hwnd, marker)| {
                let node = nodes
                    .iter()
                    .find(|n| n.hwnd == hwnd && n.marker == marker && n.interactive)?;
                (GetWindowLongPtrW(HWND(hwnd as *mut _), GWLP_USERDATA) == marker)
                    .then(|| {
                        scene
                            .catalog
                            .endpoints
                            .get(node.endpoint)
                            .map(|e| e.unit.clone())
                    })
                    .flatten()
            })
            .collect()
    }
}
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
        ui.selection_epoch
            .set(ui.selection_epoch.get().wrapping_add(1));
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
