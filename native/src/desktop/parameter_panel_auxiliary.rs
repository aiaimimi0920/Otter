//! 右侧候选视图只持唯一 Ui 的 Weak；所有点击仍交同一草稿/候选事务与执行 owner。
use super::*;
use neuro_mini_ui::PreviewGrid;
use std::rc::Weak;
const NOTIFY: u32 = WM_APP + 96;
struct Data {
    ui: Weak<Ui>,
    model: Rc<model::Model>,
    choices: Vec<Rc<controls::candidates::Choice>>,
    context: Cell<u32>,
    shown: Cell<bool>,
}
pub(super) struct Open {
    grid: PreviewGrid,
    hwnd: HWND,
    parent: HWND,
    parent_identity: isize,
    data: Rc<Data>,
}
impl Open {
    unsafe fn new(ui: &Rc<Ui>, parent: HWND) -> Result<Self> {
        let model = ui.model.borrow().clone();
        let choices = controls::candidates::choices(&model, ui.state());
        let state = ui.state().ok_or_else(|| Error::from(E_HANDLE))?;
        let data = Rc::new(Data {
            ui: Rc::downgrade(ui),
            model,
            choices,
            context: Cell::new(1),
            shown: Cell::new(false),
        });
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        let class = w!("Otter.Native.CandidateAuxiliary");
        if RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(procedure),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        }) == 0
            && GetLastError() != ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(Error::from_win32());
        }
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class,
            w!("候选"),
            WS_CHILD | WS_CLIPCHILDREN,
            0,
            0,
            1,
            1,
            Some(parent),
            None,
            Some(instance),
            Some(Rc::as_ptr(&data).cast()),
        )?;
        let result = PreviewGrid::new(
            hwnd,
            NOTIFY,
            data.choices.iter().map(|c| c.card(state)).collect(),
        );
        let grid = match result {
            Ok(grid) => grid,
            Err(error) => {
                let _ = DestroyWindow(hwnd);
                return Err(error);
            }
        };
        Ok(Self {
            grid,
            hwnd,
            parent,
            parent_identity: GetWindowLongPtrW(parent, GWLP_USERDATA),
            data,
        })
    }
    pub(super) unsafe fn hide(&self) {
        if self.data.shown.replace(false) {
            self.data
                .context
                .set(self.data.context.get().wrapping_add(1).max(1));
            if self.alive() {
                SendMessageW(self.grid.handle(), WM_CANCELMODE, None, None);
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }
    unsafe fn alive(&self) -> bool {
        IsWindow(Some(self.parent)).as_bool()
            && GetWindowLongPtrW(self.parent, GWLP_USERDATA) == self.parent_identity
            && IsWindow(Some(self.hwnd)).as_bool()
            && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) == Rc::as_ptr(&self.data) as isize
            && self.grid.alive()
    }
}
impl Drop for Open {
    fn drop(&mut self) {
        unsafe {
            self.hide();
            if self.alive() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
pub(super) unsafe fn place(
    ui: &Rc<Ui>,
    parent: HWND,
    bounds: RECT,
    target: Option<RECT>,
    dpi: u32,
) -> Result<()> {
    let mut open = ui.auxiliary.borrow().clone();
    let Some(target) = target.filter(|_| ui.available()) else {
        if let Some(open) = open {
            open.hide();
        }
        return Ok(());
    };
    let model = ui.model.borrow().clone();
    if model.candidates.is_none() {
        if let Some(open) = open {
            open.hide();
        }
        return Ok(());
    }
    if open
        .as_ref()
        .is_none_or(|o| !o.alive() || o.parent != parent || !Rc::ptr_eq(&o.data.model, &model))
    {
        // 先退役旧通知/窗口，父 strip 的销毁和目录换代不能复用旧 index。
        drop(ui.auxiliary.borrow_mut().take());
        drop(open.take());
        open = Some(Rc::new(Open::new(ui, parent)?));
        *ui.auxiliary.borrow_mut() = open.clone();
    }
    let open = open.unwrap();
    let state = ui.state().ok_or_else(|| Error::from(E_HANDLE))?;
    let enabled: Vec<_> = open
        .data
        .choices
        .iter()
        .map(|c| ui.available() && c.available(state))
        .collect();
    let selected = open.data.choices.iter().position(|c| c.selected());
    let status = selected.map_or_else(String::new, |i| {
        format!("当前 {}", open.data.choices[i].card(state).detail)
    });
    open.grid.update(
        "候选",
        &status,
        &enabled,
        selected,
        open.data.context.get(),
        dpi,
    )?;
    let rect = RECT {
        left: target.left - bounds.left,
        top: target.top - bounds.top,
        right: target.right - bounds.left,
        bottom: target.bottom - bounds.top,
    };
    let mut old = RECT::default();
    GetWindowRect(open.hwnd, &mut old)?;
    if old != target {
        SetWindowPos(
            open.hwnd,
            None,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
    }
    if !open.data.shown.replace(true) {
        let _ = ShowWindow(open.grid.handle(), SW_SHOWNOACTIVATE);
        let _ = ShowWindow(open.hwnd, SW_SHOWNOACTIVATE);
    }
    Ok(())
}
pub(super) unsafe fn translate(ui: &Rc<Ui>, msg: &MSG) -> bool {
    let Some(open) = ui
        .auxiliary
        .borrow()
        .clone()
        .filter(|o| o.alive() && o.data.shown.get())
    else {
        return false;
    };
    if msg.hwnd != open.hwnd && !IsChild(open.hwnd, msg.hwnd).as_bool() {
        return false;
    }
    if msg.message == WM_KEYDOWN && msg.wParam.0 == VK_ESCAPE.0 as usize {
        if msg.lParam.0 & (1 << 30) == 0 {
            ui.close();
            if ui.closed.get() && ui.state().is_some() {
                let _ = SetFocus(Some(ui.host));
            }
        }
        return true;
    }
    if !ui.available() {
        return desktop::session_end::blocks_input(msg.message)
            || (WM_KEYFIRST..=WM_KEYLAST).contains(&msg.message);
    }
    open.grid.translate(msg)
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Data;
    if !ptr.is_null() {
        let data = &*ptr;
        if msg == WM_NCDESTROY {
            data.shown.set(false);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        }
        if msg == WM_CLOSE {
            return LRESULT(0);
        }
        if msg == NOTIFY {
            if lp.0 == data.context.get() as isize
                && data.shown.get()
                && IsWindowVisible(hwnd).as_bool()
            {
                if let (Some(ui), Some(choice)) = (data.ui.upgrade(), data.choices.get(wp.0)) {
                    if ui.available()
                        && ui
                            .state()
                            .is_some_and(|s| data.model.current(s) && choice.available(s))
                    {
                        ui.queue(Intent::Candidate(choice.clone()));
                    }
                }
            }
            return LRESULT(0);
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
