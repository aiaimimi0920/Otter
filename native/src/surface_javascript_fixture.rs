//! Protocol fixtures shared by JS/real HWND tests; test builds only, creating no access rights or network services.
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
// Native-focus positive paths require an interactive desktop; restore only same-process windows without taking over user input queues.
pub(crate) struct Foreground {
    previous: windows::Win32::Foundation::HWND,
    target: windows::Win32::Foundation::HWND,
}
// The target thread confirms cross-process fixture activation; register only the restoration owner here without a second activation or cross-thread SetFocus.
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
// Authorization handshake for explicit native probes, not product waiting/retry; waiting must not consume the TLS single-request deadline.
// The outer 120-second deadline reaps this test process; receiving G still requires verifying the actual foreground window.
pub(crate) fn foreground_startup() {
    static SPONSORED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if std::env::var("OTTER_NATIVE_FOREGROUND_SPONSOR").as_deref() == Ok("1")
        && !SPONSORED.swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        use std::io::{Read, Write};
        // Create this thread's USER message queue first; an uninitialized suspended process cannot receive foreground authorization.
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
    // On foreground-lock rejection, do not attach external threads or synthesize activation; explicit launcher authorization does not replace the real assertion below.
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
            // Restore foreground only while this guard still owns it; after focus loss, do not overwrite the user's/another window's new choice.
            // Restore the old window if the owned target is destroyed and foreground is temporarily empty; system switching is not atomically lockable.
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
