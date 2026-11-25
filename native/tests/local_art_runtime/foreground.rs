//! A visible, enumerable test launcher must receive actual foreground activation before granting it only to this test's child process.
//! Do not attach user threads, inject global input or change the foreground lock; retain the original 15-second menu assertion.
use super::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub struct Owner {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    hwnd: HWND,
}
impl Owner {
    pub fn new() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || unsafe {
            let title = windows::core::HSTRING::from(format!(
                "Otter Art Test Input {}",
                std::process::id()
            ));
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                windows::core::w!("STATIC"),
                &title,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                420,
                40,
                440,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            tx.send(hwnd.0 as usize).unwrap();
            let mut message = MSG::default();
            while !stopped.load(Ordering::Acquire) && IsWindow(Some(hwnd)).as_bool() {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            if IsWindow(Some(hwnd)).as_bool() {
                DestroyWindow(hwnd).unwrap();
            }
        });
        let hwnd = HWND(rx.recv_timeout(Duration::from_secs(3)).unwrap() as *mut _);
        let owner = Self {
            stop,
            thread: Some(thread),
            hwnd,
        };
        owner.activate();
        owner
    }
    fn activate(&self) {
        eprintln!(
            "art-product-stage: activate-owned-launcher pid={} hwnd={:?}",
            std::process::id(),
            self.hwnd
        );
        // Normal exit revokes the previous child's foreground permission; admit each new PID again through the owned launcher.
        // If Windows refuses, the user must still activate this visible window; do not bypass the foreground lock.
        unsafe {
            let _ = SetForegroundWindow(self.hwnd);
        }
        if unsafe { GetForegroundWindow() } != self.hwnd {
            // As in the product menu, request standard UIA focus only for the owned HWND; verify actual foreground state without synthesizing activation messages.
            use windows::Win32::{System::Com::*, UI::Accessibility::*};
            let result = unsafe {
                CoCreateInstance::<_, IUIAutomation2>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
                    .and_then(|client| client.ElementFromHandle(self.hwnd))
                    .and_then(|element| element.SetFocus())
            };
            eprintln!("art-product-launcher-uia-focus: result={result:?}");
        }
        if unsafe { GetForegroundWindow() } != self.hwnd {
            eprintln!(
                "art-product-activate-needed: hwnd={} pid={}",
                self.hwnd.0 as usize,
                std::process::id()
            );
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        while unsafe { GetForegroundWindow() } != self.hwnd {
            assert!(
                Instant::now() < deadline,
                "owned launcher needs explicit foreground activation"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        eprintln!(
            "art-product-activate-satisfied: hwnd={}",
            self.hwnd.0 as usize
        );
    }
    pub fn allow(&self, product: &mut fixture::Product) {
        // CreateProcess returning does not mean the child's GUI is initialized; call once only for a PID whose owned window has been observed.
        let hwnd = ui::until(product, "owned product GUI initialized", |pid| {
            ui::windows(pid, "Otter.Native.Workspace", None, None)
                .first()
                .copied()
        });
        self.activate();
        eprintln!(
            "art-product-stage: grant-owned-foreground pid={} hwnd={hwnd:?}",
            product.0.id()
        );
        unsafe {
            AllowSetForegroundWindow(product.0.id()).unwrap();
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            // Creation or transport failures first panic the main test; cleanup must not panic again and abort the process while leaving windows behind.
            let result = thread.join();
            if !std::thread::panicking() {
                result.expect("owned foreground launcher thread failed");
            }
        }
    }
}
