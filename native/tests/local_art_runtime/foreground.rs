//! 可见、可枚举的测试启动器：须先经真实前台激活，再仅授予本测试子进程。
//! 不附加用户线程、不全局注入输入、不更改foreground lock；原菜单15秒断言仍保留。
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
        // 前一个子进程正常退出会撤销其前台许可；每个新PID都从自有启动器重新准入。
        // Windows拒绝时仍须真实激活这个可见窗口，不绕过foreground lock。
        unsafe {
            let _ = SetForegroundWindow(self.hwnd);
        }
        if unsafe { GetForegroundWindow() } != self.hwnd {
            // 和产品菜单一致，只请求自有HWND的标准UIA焦点；仍以真实前台确认，不合成激活消息。
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
        // CreateProcess返回不代表子进程已初始化GUI；只对已观察到自有窗口的PID调用一次。
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
            // 创建/收发失败会先让主测试panic；清理不能再panic导致进程abort和窗口遗留。
            let result = thread.join();
            if !std::thread::panicking() {
                result.expect("owned foreground launcher thread failed");
            }
        }
    }
}
