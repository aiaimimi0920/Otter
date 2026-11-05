//! 缩小隐藏面板并投递滚动消息，验证末尾控件可达；不注入全局输入或改变显示器配置。
use super::*;
use windows::core::{Error, Result};
struct Restore(HWND, RECT);
impl Drop for Restore {
    fn drop(&mut self) {
        unsafe {
            let _ = SetWindowPos(
                self.0,
                None,
                self.1.left,
                self.1.top,
                self.1.right - self.1.left,
                self.1.bottom - self.1.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}
fn require(value: bool, message: &'static str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}
unsafe fn contained(panel: HWND, button: HWND) -> Result<()> {
    let mut rect = RECT::default();
    let mut client = RECT::default();
    GetWindowRect(button, &mut rect)?;
    GetClientRect(panel, &mut client)?;
    let mut start = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut end = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    require(
        ScreenToClient(panel, &mut start).as_bool() && ScreenToClient(panel, &mut end).as_bool(),
        "Scroll coordinate mapping failed",
    )?;
    require(
        start.x >= 0 && start.y >= 0 && end.x <= client.right && end.y <= client.bottom,
        "Scrolled control is clipped",
    )
}
pub(in crate::desktop::tool_panel) unsafe fn run(panel: HWND) -> Result<()> {
    let mut original = RECT::default();
    GetWindowRect(panel, &mut original)?;
    let _restore = Restore(panel, original);
    let dpi = GetDpiForWindow(panel).max(96) as i32;
    SetWindowPos(
        panel,
        None,
        0,
        0,
        160 * dpi / 96,
        140 * dpi / 96,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
    )?;
    SendMessageW(panel, WM_VSCROLL, Some(WPARAM(SB_BOTTOM.0 as usize)), None);
    SendMessageW(panel, WM_HSCROLL, Some(WPARAM(SB_BOTTOM.0 as usize)), None);
    require(
        GetScrollPos(panel, SB_VERT) > 0 && GetScrollPos(panel, SB_HORZ) > 0,
        "Panel did not scroll both axes",
    )?;
    let last_id = crate::desktop::tool_panel_actions::ACTIONS
        .last()
        .unwrap()
        .0;
    let last = GetDlgItem(Some(panel), last_id as i32)?;
    reveal(panel, last);
    contained(panel, last)?;
    SendMessageW(panel, WM_VSCROLL, Some(WPARAM(SB_TOP.0 as usize)), None);
    SendMessageW(panel, WM_HSCROLL, Some(WPARAM(SB_TOP.0 as usize)), None);
    require(
        GetScrollPos(panel, SB_VERT) == 0 && GetScrollPos(panel, SB_HORZ) == 0,
        "Panel did not return to origin",
    )?;
    let first = GetDlgItem(
        Some(panel),
        crate::desktop::tool_commands::TOOLS[0].id as i32,
    )?;
    contained(panel, first)?;
    drop(_restore);
    let mut restored = RECT::default();
    GetWindowRect(panel, &mut restored)?;
    require(restored == original, "Panel size was not restored")?;
    println!("native-panel-scroll-smoke: horizontal=true vertical=true reveal=true restored=true");
    Ok(())
}
