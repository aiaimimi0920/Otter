//! JS/真实 HWND 测试共用的协议夹具；只有测试构建可用，不创建访问权或网络服务。
use crate::surface_live_binding::Binding;
use serde_json::{json, Value};
pub(crate) static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-counter.js");
pub(crate) fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:javascript","generation":7},"attachments":{"attachment:javascript":{"descriptor":{"attachmentId":"attachment:javascript","instanceId":"instance:javascript","hookNodeId":"node:javascript","deviceId":"viewer:javascript"},"lifecycle":"active","lifecycleRevision":1}}}}"#,
        "https://localhost:1234", "viewer:javascript", "node:javascript").unwrap()
}
pub(crate) fn snapshot(revision: u64, quote: i32) -> Value {
    json!({"revision":revision,"scene":{"id":"root","type":"column","children":[
        {"id":"refresh","type":"button","events":{"click":"refresh"}}
    ]},"state":{"quote":quote}})
}
pub(crate) fn resources() -> Value {
    json!({"logo":"data:image/png;base64,fixture"})
}
pub(crate) fn text(scene: &crate::surface_scene::Scene) -> &str {
    scene.root().children()[0].props()["text"].as_str().unwrap()
}
// 原生焦点 positive path 必须在交互桌面验证；仅恢复同进程窗口，不接管用户输入队列。
pub(crate) struct Foreground {
    previous: windows::Win32::Foundation::HWND,
    target: windows::Win32::Foundation::HWND,
}
// 跨进程夹具由目标线程确认激活；此处只登记恢复 owner，不发起第二次激活或跨线程 SetFocus。
pub(crate) unsafe fn guard_foreground(parent: windows::Win32::Foundation::HWND) -> Foreground {
    let previous = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
    Foreground {
        previous: if owned_window(previous) {
            previous
        } else {
            windows::Win32::Foundation::HWND::default()
        },
        target: parent,
    }
}
unsafe fn owned_window(hwnd: windows::Win32::Foundation::HWND) -> bool {
    let mut pid = 0;
    let thread =
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
    thread != 0 && pid == std::process::id()
}
// 显式原生探针的启动前提，不是产品等待/重试。Application 必须在 TLS 夹具计时前调用；
// 外层只持有本测试进程并以 120 秒就绪期限回收，收到 G 也仍须验证真正前台。
pub(crate) fn foreground_startup() {
    static SPONSORED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if std::env::var("OTTER_NATIVE_FOREGROUND_SPONSOR").as_deref() == Ok("1")
        && !SPONSORED.swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        use std::io::{Read, Write};
        // 先建立本线程 USER 消息队列；尚未初始化的挂起进程不能接收前台授权。
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            let _ = PeekMessageW(&mut MSG::default(), None, 0, 0, PM_NOREMOVE);
        }
        println!("OTTER_FOREGROUND_READY");
        std::io::stdout().flush().unwrap();
        let mut permission = [0];
        std::io::stdin().read_exact(&mut permission).unwrap();
        assert_eq!(permission, *b"G");
    }
}
#[track_caller]
pub(crate) unsafe fn focus_foreground(parent: windows::Win32::Foundation::HWND) -> Foreground {
    use windows::Win32::UI::{Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::*};
    assert!(
        owned_window(parent),
        "focus fixture requires its own window"
    );
    foreground_startup();
    let guard = guard_foreground(parent);
    SetWindowPos(
        parent,
        None,
        -32000,
        -32000,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
    let _ = ShowWindow(parent, SW_SHOWNOACTIVATE);
    let _ = SetForegroundWindow(parent);
    // 前台锁拒绝时不关联外部线程或合成激活；显式启动器授权也不能替代下方真实断言。
    let _ = SetFocus(Some(parent));
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "focus fixture requires an interactive desktop"
    );
    guard
}
impl Drop for Foreground {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::*;
        unsafe {
            let current = GetForegroundWindow();
            // 只归还仍由本guard持有的前台；失焦后不覆盖用户/其他窗口的新选择。
            // 自有target已销毁且前台暂空时仍恢复旧窗口；系统切换不是可原子锁定的资源。
            if owned_window(self.previous)
                && (current == self.target
                    || (current.0.is_null() && !IsWindow(Some(self.target)).as_bool()))
            {
                let _ = SetForegroundWindow(self.previous);
            }
        }
    }
}

#[cfg(test)]
mod foreground_ownership_tests {
    use super::*;
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

    #[test]
    fn invalid_and_system_windows_are_not_owned_by_the_test() {
        unsafe {
            assert!(!owned_window(HWND::default()));
            assert!(!owned_window(GetDesktopWindow()));
        }
    }

    #[test]
    fn hidden_owned_window_is_accepted_and_foreign_previous_is_not_retained() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                windows::core::w!("STATIC"),
                windows::core::w!("Otter owned foreground safety fixture"),
                WS_POPUP,
                -32000,
                -32000,
                40,
                40,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            assert!(owned_window(hwnd));
            let guard = guard_foreground(hwnd);
            assert!(guard.previous.0.is_null() || owned_window(guard.previous));
            drop(guard);
            DestroyWindow(hwnd).unwrap();
        }
    }

    #[test]
    fn foreign_target_is_rejected_before_startup_or_window_operations() {
        let result = std::panic::catch_unwind(|| unsafe { focus_foreground(GetDesktopWindow()) });
        assert!(result.is_err());
    }
}
