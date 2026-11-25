//! Actual product EXE under the production Job/pipe policy, with only owned offscreen windows.
#![cfg(windows)]
use otter_runtime::{live_browser_scroll, live_source_uia, live_source_uia_test_window::Window};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

fn product() -> crate::native_product::Product {
    crate::native_product::Product::resolve(Path::new(env!("CARGO_BIN_EXE_otter-native")))
        .expect("resolve exact Live worker product")
}

fn pump_until<T>(worker: thread::JoinHandle<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(6);
    while !worker.is_finished() {
        assert!(Instant::now() < deadline, "Live worker fixture deadline");
        Window::pump();
        thread::sleep(Duration::from_millis(1));
    }
    worker.join().unwrap()
}

#[test]
fn product_observation_worker_reads_owned_edit_without_password() {
    let product = product();
    let window = Window::new();
    let target = otter_runtime::live_window_target::Target::bind(window.hwnd).unwrap();
    let samples = pump_until(thread::spawn(move || {
        let mut worker =
            live_source_uia::process::Worker::start_product_for_test(target, product.path())
                .expect("product observation worker startup");
        worker.scan(&|| false).expect("product observation scan")
    }));
    let edits: Vec<_> = samples
        .iter()
        .filter(|sample| sample.locator.control_type == "Edit")
        .collect();
    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0].value.as_ref().unwrap()["value"]["text"],
        "value-seven"
    );
    assert!(!serde_json::to_string(&samples)
        .unwrap()
        .contains("password-secret"));
}

struct ChromiumWindow(HWND);
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
impl ChromiumWindow {
    fn new() -> Self {
        unsafe {
            let instance = GetModuleHandleW(None).unwrap().into();
            let class = w!("Chrome_WidgetWin_1");
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(window_proc),
                    hInstance: instance,
                    lpszClassName: class,
                    ..Default::default()
                }),
                0
            );
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class,
                w!("Otter owned scroll fixture"),
                WS_POPUP,
                -20000,
                -20000,
                120,
                80,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Self(hwnd)
        }
    }
}
impl Drop for ChromiumWindow {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0).unwrap() };
    }
}

#[test]
fn product_browser_scroll_worker_completes_uia_handshake() {
    let product = product();
    let window = ChromiumWindow::new();
    let target = otter_runtime::live_window_target::Target::bind(window.0).unwrap();
    assert!(live_browser_scroll::eligible(&target));
    pump_until(thread::spawn(move || {
        // The ready reply follows identity checks and UIA initialization, not just process spawn.
        let _worker = live_browser_scroll::Worker::start_product_for_test(target, product.path())
            .expect("product Chromium scroll worker handshake");
    }));
}
