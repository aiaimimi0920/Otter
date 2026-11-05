//! 窗口过程不回调业务；消息交给外层级联菜单控制器，避免重入编辑模型。
use super::*;
fn push(data: &Data, event: Event) {
    if data.failed.get() {
        return;
    }
    let mut events = data.events.borrow_mut();
    if matches!(event, Event::Move(..)) && matches!(events.back(), Some(Event::Move(..))) {
        events.pop_back();
    }
    if events.len() >= 64 {
        events.clear();
        data.failed.set(true); // 输入过载时取消，不能继续执行截断的点击序列。
    } else {
        events.push_back(event);
    }
}
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    pointer: usize,
) -> LRESULT {
    let data = &*(pointer as *const Data);
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let x = lp.0 as i16 as i32;
        let y = (lp.0 >> 16) as i16 as i32;
        match msg {
            WM_GETOBJECT if lp.0 == windows::Win32::UI::Accessibility::UiaRootObjectId as isize => {
                // 释放 RefCell 借用再调用 UIA；系统调用可以重入销毁窗口。
                let provider = accessibility::provider(data, hwnd);
                if let Some(provider) = provider {
                    windows::Win32::UI::Accessibility::UiaReturnRawElementProvider(
                        hwnd, wp, lp, &provider,
                    )
                } else {
                    LRESULT(0)
                }
            }
            WM_PAINT => {
                draw::paint(hwnd, data);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_MOUSEMOVE => {
                push(data, Event::Move(x, y));
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                push(data, Event::Down(x, y));
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                push(data, Event::Up(x, y));
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                push(data, Event::Wheel((wp.0 >> 16) as i16));
                LRESULT(0)
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                push(data, Event::Key(wp.0 as u32));
                LRESULT(0)
            }
            WM_CANCELMODE | WM_CAPTURECHANGED | WM_CLOSE => {
                push(data, Event::Cancel);
                LRESULT(0)
            }
            WM_DPICHANGED => {
                push(data, Event::Dpi);
                LRESULT(0)
            }
            WM_NCDESTROY => {
                data.accessibility.revoke();
                data.provider.borrow_mut().take();
                windows::Win32::UI::Accessibility::UiaReturnRawElementProvider(
                    hwnd,
                    WPARAM(0),
                    LPARAM(0),
                    None,
                );
                data.destroyed.set(true);
                // 唤醒可能正在 GetMessage 内派发销毁通知的菜单循环；不传递裸指针。
                let _ = PostThreadMessageW(GetCurrentThreadId(), WM_NULL, WPARAM(0), LPARAM(0));
                push(data, Event::Cancel);
                let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
                DefSubclassProc(hwnd, msg, wp, lp)
            }
            _ => DefSubclassProc(hwnd, msg, wp, lp),
        }
    }))
    .unwrap_or_else(|_| {
        data.failed.set(true);
        LRESULT(0)
    })
}
