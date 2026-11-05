//! Number 粘贴事务：取同一份快照交 EM_REPLACESEL；延迟渲染重入改变目标即作废。
#[path = "surface_number_clipboard.rs"]
mod clipboard;
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        UI::{Controls::*, Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
    },
};
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Number.Paste.Owner");
struct Guard {
    hwnd: HWND,
    marker: Box<u8>,
}
impl Guard {
    unsafe fn new(hwnd: HWND) -> Option<Self> {
        if !GetPropW(hwnd, OWNER).is_invalid() {
            return None;
        }
        let guard = Self {
            hwnd,
            marker: Box::new(0),
        };
        SetPropW(
            hwnd,
            OWNER,
            Some(HANDLE((&*guard.marker as *const u8).cast_mut().cast())),
        )
        .ok()?;
        Some(guard)
    }
    unsafe fn current(&self) -> bool {
        GetPropW(self.hwnd, OWNER).0 == (&*self.marker as *const u8).cast_mut().cast()
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            if self.current() {
                let _ = RemovePropW(self.hwnd, OWNER);
            }
        }
    }
}
pub unsafe fn invalidate(hwnd: HWND) {
    let _ = RemovePropW(hwnd, OWNER);
}
unsafe fn eligible(hwnd: HWND) -> bool {
    IsWindowEnabled(hwnd).as_bool()
        && GetWindowLongW(hwnd, GWL_STYLE) as u32 & ES_READONLY as u32 == 0
        && GetParent(hwnd).is_ok_and(|parent| {
            SendMessageW(
                parent,
                super::CAN_PASTE,
                None,
                Some(LPARAM(hwnd.0 as isize)),
            )
            .0 != 0
        })
}
pub unsafe fn paste(hwnd: HWND) {
    paste_with(hwnd, || clipboard::read(hwnd));
}
pub(crate) unsafe fn paste_with(hwnd: HWND, read: impl FnOnce() -> Option<Vec<u16>>) {
    if !eligible(hwnd) {
        return;
    }
    let Some(guard) = Guard::new(hwnd) else {
        return;
    };
    let Some(mut text) = read() else {
        return;
    };
    // GetClipboardData 可触发外部延迟渲染；此时编辑、选区、焦点、权限/组合变化
    // 会清临时 marker。Box 仍存活，旧 HWND 被重用也不能冒充这个操作。
    if !guard.current()
        || !eligible(hwnd)
        || text.len() > super::super::surface_edit_control::MAX_UNITS
        || text.contains(&0)
    {
        return;
    }
    text.push(0);
    // 资格查询当前无副作用；仍复查 marker，避免后续接线增加重入后使用过期结论。
    if !guard.current() {
        return;
    }
    // read 返回时剪贴板已关闭；只使用复制出的同一快照，不再次发送 WM_PASTE。
    SendMessageW(
        hwnd,
        EM_REPLACESEL,
        Some(WPARAM(1)),
        Some(LPARAM(text.as_ptr() as isize)),
    );
}
