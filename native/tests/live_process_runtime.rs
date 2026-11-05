//! 显式跨进程验证；子进程是本测试二进制，只捕获它创建的纯色窗口。
#![cfg(windows)]
use otter_native::{live_job::LiveJob, live_window_target::Target};
use std::{
    io::{BufRead, BufReader, Read},
    os::windows::io::AsRawHandle,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::DwmFlush, Gdi::*},
        System::{Com::*, LibraryLoader::GetModuleHandleW, Threading::WaitForSingleObject},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

struct FixtureProcess {
    child: Child,
    reader: Option<JoinHandle<()>>,
    hwnd: HWND,
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
impl FixtureProcess {
    fn start() -> Self {
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "live_fixture_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("OTTER_NATIVE_LIVE_CHILD", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut owner = Self {
            child,
            reader: None,
            hwnd: HWND::default(),
        };
        let output = owner.child.stdout.take().unwrap();
        let (send, ready) = mpsc::sync_channel(1);
        owner.reader = Some(std::thread::spawn(move || {
            // 只解析自己启动的测试进程，不信任标题或枚举结果来选择目标。
            for line in BufReader::new(output)
                .take(64 * 1024)
                .lines()
                .map_while(Result::ok)
            {
                if let Some((_, value)) = line.split_once("OTTER_LIVE_FIXTURE_HWND=") {
                    if let Ok(value) = usize::from_str_radix(value.trim(), 16) {
                        let _ = send.try_send(value);
                    }
                }
            }
        }));
        owner.hwnd = HWND(
            ready
                .recv_timeout(Duration::from_secs(5))
                .expect("fixture child did not become ready") as *mut _,
        );
        unsafe {
            let mut process = 0;
            GetWindowThreadProcessId(owner.hwnd, Some(&mut process));
            assert_eq!(
                process,
                owner.child.id(),
                "refusing to capture a window outside the fixture process"
            );
        }
        owner
    }
    fn finish(&mut self, success: bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.success(), success);
                self.reader.take().unwrap().join().unwrap();
                return;
            }
            assert!(Instant::now() < deadline, "fixture child did not exit");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for FixtureProcess {
    fn drop(&mut self) {
        // 仅终止本对象持有的子进程 handle，不按进程名/PID 扫描或结束其它应用。
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
unsafe fn pump() {
    let mut message = MSG::default();
    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}
#[test]
#[ignore = "requires interactive desktop; captures only its own fixture child process"]
fn actual_wgc_external_process_normal_and_forced_exit() {
    // 与实际桌面应用一致：UI STA 在所有采集 worker 之前建立、之后释放。
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _apartment = Apartment;
    for forced in [false, true] {
        let mut fixture = FixtureProcess::start();
        let target = Target::bind(fixture.hwnd).unwrap();
        let (mut job, mut consumer) = LiveJob::start_window_bound(target, Ok).unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        let first = loop {
            unsafe {
                pump();
            }
            if let Some(delivery) = consumer
                .try_next()
                .expect("fixture source closed before first frame")
            {
                break delivery.frame;
            }
            assert!(
                Instant::now() < deadline,
                "fixture capture timed out: {:?}",
                job.take_ready()
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(first.resource.width > 0 && first.resource.height > 0);
        let weak = std::sync::Arc::downgrade(&first);
        drop(first);
        if forced {
            fixture.child.kill().unwrap();
        } else {
            unsafe {
                PostMessageW(Some(fixture.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
            }
        }
        // 此处故意不泵 WinEvent：真实进程句柄能独立报告退出，不能依赖销毁通知。
        assert_eq!(
            unsafe {
                WaitForSingleObject(
                    HANDLE(job.source_process_handle().unwrap().as_raw_handle()),
                    3000,
                )
            },
            WAIT_OBJECT_0
        );
        let mut result = job.take_ready();
        assert!(
            job.source_process_handle().is_none(),
            "retired source must leave the wait set"
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while result.is_none() {
            unsafe {
                pump();
            }
            result = job.take_ready();
            assert!(Instant::now() < deadline, "capture worker did not exit");
            std::thread::sleep(Duration::from_millis(5));
        }
        let error = result
            .unwrap()
            .expect_err("source exit must preserve a failure reason");
        assert!(
            error.contains("TargetChanged") || error.contains("SourceClosed"),
            "{error}"
        );
        assert!(job.completion_handle().is_none());
        assert!(weak.upgrade().is_none());
        fixture.finish(!forced);
        println!("native-live-process-runtime: forced={forced} frame=true process-wake=true retired=true joined=true");
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, message, wp, lp)
}
#[test]
#[ignore = "fixture helper: launch only from actual_wgc_external_process_normal_and_forced_exit"]
fn live_fixture_child() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_LIVE_CHILD").as_deref(),
        Ok("1"),
        "fixture requires parent harness"
    );
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.LiveProcessFixture"),
                lpfnWndProc: Some(procedure),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            }),
            0
        );
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("Otter.Native.LiveProcessFixture"),
            w!("Otter Live process fixture"),
            WS_POPUP,
            20,
            20,
            128,
            96,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap();
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(hwnd);
        DwmFlush().unwrap();
        println!("OTTER_LIVE_FIXTURE_HWND={:X}", hwnd.0 as usize);
        while IsWindow(Some(hwnd)).as_bool() {
            let mut message = MSG::default();
            let result = GetMessageW(&mut message, None, 0, 0).0;
            assert!(result >= 0);
            if result == 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if IsWindow(Some(hwnd)).as_bool() {
            DestroyWindow(hwnd).unwrap();
        }
        UnregisterClassW(w!("Otter.Native.LiveProcessFixture"), Some(instance)).unwrap();
    }
}
