//! 多行 child 只借用同一个参数事务 owner；滚动/折叠不重建 EDIT 或丢弃草稿。
use super::*;
use std::rc::Weak;

#[derive(Clone)]
pub(in crate::desktop) struct Inline(Weak<Ui>);
impl Inline {
    pub(in crate::desktop) unsafe fn hide_auxiliary(&self) {
        let open = self
            .0
            .upgrade()
            .and_then(|ui| ui.auxiliary.borrow().clone());
        if let Some(open) = open {
            open.hide();
        }
    }
    pub(in crate::desktop) unsafe fn place_auxiliary(
        &self,
        parent: HWND,
        bounds: RECT,
        target: Option<RECT>,
        dpi: u32,
    ) -> Result<()> {
        if let Some(ui) = self.0.upgrade().filter(|ui| !ui.closed.get()) {
            auxiliary::place(&ui, parent, bounds, target, dpi)?;
        }
        Ok(())
    }
    pub(in crate::desktop) unsafe fn idle(&self) -> bool {
        self.0.upgrade().is_none_or(|ui| {
            let empty = ui.pending.borrow().is_empty();
            empty && ui.drafts_idle()
        })
    }
    pub(in crate::desktop) unsafe fn context(&self, id: &str, value: bool) {
        if let Some(ui) = self.0.upgrade().filter(|ui| !ui.closed.get()) {
            inline_input::context(&ui, id, value);
        }
    }
    pub(in crate::desktop) unsafe fn place(
        &self,
        bounds: RECT,
        rows: Vec<(String, RECT, RECT)>,
    ) -> Result<()> {
        let Some(ui) = self.0.upgrade().filter(|ui| !ui.closed.get()) else {
            return Ok(());
        };
        if ui.state().is_none() || GetWindowLongPtrW(ui.parent, GWLP_USERDATA) != ui.parent_identity
        {
            return Err(Error::from(E_HANDLE));
        }
        if rows.is_empty() {
            ui.inline_rows.borrow_mut().clear();
            let _ = ShowWindow(ui.hwnd.get(), SW_HIDE);
            return Ok(());
        }
        let rows: Vec<_> = rows
            .into_iter()
            .map(|(id, mut row, mut clip)| {
                for rect in [&mut row, &mut clip] {
                    rect.left -= bounds.left;
                    rect.right -= bounds.left;
                    rect.top -= bounds.top;
                    rect.bottom -= bounds.top;
                }
                (id, row, clip)
            })
            .collect();
        let mut current = RECT::default();
        GetClientRect(ui.hwnd.get(), &mut current)?;
        if *ui.inline_rows.borrow() == rows
            && current.right == bounds.right - bounds.left
            && current.bottom == bounds.bottom - bounds.top
        {
            if !IsWindowVisible(ui.hwnd.get()).as_bool() {
                let _ = ShowWindow(ui.hwnd.get(), SW_SHOWNOACTIVATE);
            }
            return Ok(());
        }
        let redraw = neuro_mini_ui::paint_batch::Hold::new(ui.hwnd.get());
        *ui.inline_rows.borrow_mut() = rows;
        SetWindowPos(
            ui.hwnd.get(),
            None,
            0,
            0,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOREDRAW,
        )?;
        if ui.closed.get() || ui.state().is_none() {
            return Ok(());
        }
        place(&ui);
        drop(redraw);
        if !ui.closed.get() && ui.state().is_some() {
            let _ = ShowWindow(ui.hwnd.get(), SW_SHOWNOACTIVATE);
            neuro_mini_ui::paint_batch::present(ui.hwnd.get())?;
        }
        Ok(())
    }
    pub(in crate::desktop) unsafe fn focus(&self, id: &str) {
        let Some(ui) = self.0.upgrade().filter(|ui| ui.available()) else {
            return;
        };
        if let Some(child) = focus_child(&ui, id) {
            let _ = SetFocus(Some(child));
        }
    }
}
pub(super) unsafe fn focus_child(ui: &Ui, id: &str) -> Option<HWND> {
    ui.bindings
        .borrow()
        .iter()
        .find(|b| {
            // Only the final component is a fixed control role; a row ID may contain ':'.
            b.key.rsplit_once(':').is_some_and(|(row, _)| row == id)
                && b.enabled
                && IsWindowVisible(b.hwnd).as_bool()
        })
        .map(|b| b.hwnd)
}
impl Panel {
    pub(in crate::desktop) unsafe fn inline(
        host: HWND,
        state: &State,
        parent: HWND,
    ) -> Result<Self> {
        Self::create(host, state, Some(parent))
    }
    pub(in crate::desktop) fn inline_binding(&self) -> Option<Inline> {
        self.ui.inline.then(|| Inline(Rc::downgrade(&self.ui)))
    }
}
pub(super) unsafe fn wheel(ui: &Ui, wp: WPARAM, lp: LPARAM) -> bool {
    // wheel 的 LPARAM 已是屏幕坐标，不通过图片缩放或独立表单滚动补偿。
    SendMessageW(ui.parent, WM_MOUSEWHEEL, Some(wp), Some(lp));
    true
}
pub(super) unsafe fn place(ui: &Ui) {
    let rows = ui.inline_rows.borrow().clone();
    let model = ui.model.borrow().clone();
    let placements = ui.placements.borrow().clone();
    let dpi = GetDpiForWindow(ui.parent).max(96);
    let region = CreateRectRgn(0, 0, 0, 0);
    if region.is_invalid() {
        ui.error("无法裁剪参数栏");
        return;
    }
    // 根 HRGN 只包含真实控件，不盖住 label、眼睛、anchor 或行间空隙。
    for (child, rect) in placements {
        let index = (rect.top.max(0) / 36) as usize;
        let row = model
            .rows
            .get(index)
            .and_then(|field| rows.iter().find(|(id, _, _)| *id == field.id));
        if let Some((_, row, clip)) = row {
            let local = layout::scaled(
                RECT {
                    top: rect.top % 36,
                    bottom: rect.bottom - index as i32 * 36,
                    ..rect
                },
                dpi,
            );
            let target = RECT {
                left: row.left + local.left,
                top: row.top + local.top,
                right: row.left + local.right,
                bottom: row.top + local.bottom,
            };
            let _ = SetWindowPos(
                child,
                None,
                target.left,
                target.top,
                target.right - target.left,
                target.bottom - target.top,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOREDRAW,
            );
            let _ = ShowWindow(child, SW_SHOWNOACTIVATE);
            let left = target.left.max(clip.left);
            let right = target.right.min(clip.right);
            let top = target.top.max(clip.top);
            let bottom = target
                .bottom
                .min(target.top + (24 * dpi / 96) as i32)
                .min(clip.bottom);
            if left >= right || top >= bottom {
                continue;
            }
            let part = CreateRectRgn(left, top, right, bottom);
            if !part.is_invalid() {
                CombineRgn(Some(region), Some(region), Some(part), RGN_OR);
                let _ = DeleteObject(HGDIOBJ(part.0));
            }
        } else {
            let _ = ShowWindow(child, SW_HIDE);
        }
    }
    if SetWindowRgn(ui.hwnd.get(), Some(region), false) == 0 {
        let _ = DeleteObject(HGDIOBJ(region.0));
    }
}
