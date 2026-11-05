//! 面板子按钮按当前 DIP/滚动几何直接创建；布局只定位发生变化的真实 HWND。
//! 成功发布前由临时 Window 集合拥有，失败回滚；发布后交给父面板，不缓存几何。
use super::{entries, geometry, scroll, Window};
use windows::{
    core::{w, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::ScreenToClient,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub(super) unsafe fn create(panel: HWND, instance: HINSTANCE) -> Result<Vec<Window>> {
    create_checked(panel, instance, |_| Ok(()))
}

pub(super) unsafe fn create_checked(
    panel: HWND,
    instance: HINSTANCE,
    mut before: impl FnMut(usize) -> Result<()>,
) -> Result<Vec<Window>> {
    let dpi = GetDpiForWindow(panel);
    // 范围来自固定目录和客户区，不依赖按钮已存在；保留最终 layout 处理 DPI/尺寸变化。
    let offset = scroll::sync(panel);
    let mut controls = Vec::with_capacity(entries().count());
    for (index, (id, label)) in entries().enumerate() {
        before(index)?;
        let rect = geometry::button(index, crate::desktop::tool_commands::TOOLS.len(), dpi);
        controls.push(Window(CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            &HSTRING::from(label.replace('\t', " ")),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            rect.left - offset.x,
            rect.top - offset.y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(panel),
            Some(HMENU(id as *mut _)),
            Some(instance),
            None,
        )?));
    }
    Ok(controls)
}

unsafe fn matches(button: HWND, parent: HWND, expected: RECT, offset: POINT) -> bool {
    let mut actual = RECT::default();
    if GetWindowRect(button, &mut actual).is_err() {
        return false;
    }
    let mut origin = POINT {
        x: actual.left,
        y: actual.top,
    };
    ScreenToClient(parent, &mut origin).as_bool()
        && origin.x == expected.left - offset.x
        && origin.y == expected.top - offset.y
        && i64::from(actual.right) - i64::from(actual.left)
            == i64::from(expected.right - expected.left)
        && i64::from(actual.bottom) - i64::from(actual.top)
            == i64::from(expected.bottom - expected.top)
}

pub(super) unsafe fn layout(panel: HWND) {
    let dpi = GetDpiForWindow(panel);
    let offset = scroll::sync(panel);
    for (index, (id, _)) in entries().enumerate() {
        let rect = geometry::button(index, crate::desktop::tool_commands::TOOLS.len(), dpi);
        if let Ok(button) = GetDlgItem(Some(panel), id as i32) {
            if matches(button, panel, rect, offset) {
                continue;
            }
            // 读取失败时仍沿既有 best-effort 定位路径；真实 resize/滚动不能被缓存吞掉。
            let _ = SetWindowPos(
                button,
                None,
                rect.left - offset.x,
                rect.top - offset.y,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}
