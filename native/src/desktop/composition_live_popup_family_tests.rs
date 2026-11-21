//! 附属弹窗几何与真实 WGC 会话生命周期回归；仅创建和操作自有测试窗口。
use super::*;
#[path = "composition_live_popup_shutdown_tests.rs"]
mod shutdown_tests;
use windows::core::w;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::{Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW};
thread_local! { static CLICKS: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) }; }
unsafe extern "system" fn source_proc(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if GetWindowLongPtrW(hwnd, GWLP_USERDATA) == 1 {
        CLICKS.with(|clicks| {
            let (down, up) = clicks.get();
            if message == WM_LBUTTONDOWN {
                clicks.set((down + 1, up));
            }
            if message == WM_LBUTTONUP {
                clicks.set((down, up + 1));
            }
        });
    }
    DefWindowProcW(hwnd, message, wp, lp)
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}
#[test]
fn maps_entire_popup_beyond_source_crop_and_scaled_output() {
    assert_eq!(
        map_rect(
            rect(100, 200, 100, 50),
            rect(800, 300, 200, 100),
            rect(180, 210, 120, 80)
        ),
        Some(rect(960, 320, 240, 160))
    );
    assert_eq!(
        map_rect(
            rect(-500, -300, 100, 100),
            rect(-100, 0, 50, 50),
            rect(-520, -320, 100, 100)
        ),
        Some(rect(-110, -10, 50, 50))
    );
}
#[test]
fn rejects_empty_and_overflowing_popup_geometry() {
    assert!(map_rect(rect(0, 0, 0, 10), rect(0, 0, 10, 10), rect(0, 0, 5, 5)).is_none());
    assert!(map_rect(
        rect(0, 0, 1, 1),
        rect(0, 0, 100, 100),
        rect(i32::MAX - 10, 0, 5, 5)
    )
    .is_none());
}

#[test]
fn popup_mirror_follows_owner_topmost_band_without_activating() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    for topmost in [false, true] {
        for noactivate in [false, true] {
            let frames = crate::live_frames::Source::new();
            let owner = Source::with_style(
                None,
                rect(0, 0, 100, 100),
                if topmost {
                    WS_EX_TOPMOST
                } else {
                    WINDOW_EX_STYLE::default()
                },
            );
            let mirror = LiveWindow::new(frames.subscribe().unwrap(), 40, 40).unwrap();
            unsafe {
                let foreground = GetForegroundWindow();
                // 只测试镜像配置；不依赖无帧 Live owner 的置顶切换是否已提交。
                assert_eq!(
                    GetWindowLongPtrW(owner.0, GWL_EXSTYLE) & WS_EX_TOPMOST.0 as isize != 0,
                    topmost,
                    "owner topmost setup was not applied"
                );
                mirror.configure_popup(owner.0, noactivate).unwrap();
                assert_eq!(GetWindow(mirror.popup_hwnd(), GW_OWNER).unwrap(), owner.0);
                let style = GetWindowLongPtrW(mirror.popup_hwnd(), GWL_EXSTYLE);
                assert_eq!(style & WS_EX_TOPMOST.0 as isize != 0, topmost);
                assert_eq!(style & WS_EX_NOACTIVATE.0 as isize != 0, noactivate);
                assert_eq!(GetForegroundWindow(), foreground);
                let mut affinity = 0;
                GetWindowDisplayAffinity(mirror.popup_hwnd(), &mut affinity).unwrap();
                assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
            }
        }
    }
}

#[test]
fn root_completion_retains_a_wait_handle_for_delayed_popup_cleanup() {
    use std::{
        os::windows::io::AsRawHandle,
        sync::{atomic::AtomicUsize, mpsc, Arc},
    };
    use windows::Win32::System::Threading::WaitForSingleObject;
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let (root_release, root_wait) = mpsc::channel();
    let (popup_release, popup_wait) = mpsc::channel();
    let (started, starting) = mpsc::channel();
    let root_started = started.clone();
    let create = |consumer| LiveWindow::new(consumer, 100, 100).map_err(io::Error::other);
    let (job, window) = LiveJob::spawn_bound(&active, create, move |_, _| {
        root_started.send(()).unwrap();
        root_wait.recv_timeout(Duration::from_secs(10)).unwrap();
        Ok(())
    })
    .unwrap();
    let mut root = Session::new(job, window);
    let (job, window) = LiveJob::spawn_bound(&active, create, move |_, _| {
        started.send(()).unwrap();
        popup_wait.recv_timeout(Duration::from_secs(10)).unwrap();
        Ok(())
    })
    .unwrap();
    root.popups.members.push(Member {
        hwnd: 0,
        placement: rect(0, 0, 100, 100),
        session: Session::new(job, window),
    });
    // CaptureJob 可在 work 入口前响应取消；本用例必须先确保两个延迟 worker 已进入。
    for _ in 0..2 {
        starting.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    root.stop();
    root_release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while root.capture_result.is_none() {
        assert!(root.poll().is_none());
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(root.job.completion_handle().is_none());
    let handle = root
        .completion_handle()
        .expect("child cleanup needs a host wait owner");
    let handle = HANDLE(handle.as_raw_handle());
    assert_eq!(unsafe { WaitForSingleObject(handle, 0) }, WAIT_TIMEOUT);
    popup_release.send(()).unwrap();
    assert_eq!(unsafe { WaitForSingleObject(handle, 5000) }, WAIT_OBJECT_0);
    assert_eq!(root.poll(), Some(Ok(())));
    assert!(root.popups.is_empty());
}

struct Source(HWND);
impl Source {
    fn new(owner: Option<HWND>, screen: RECT, noactivate: bool) -> Self {
        Self::with_style(
            owner,
            screen,
            if noactivate {
                WS_EX_NOACTIVATE
            } else {
                WINDOW_EX_STYLE::default()
            },
        )
    }
    fn with_style(owner: Option<HWND>, screen: RECT, style: WINDOW_EX_STYLE) -> Self {
        static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        REGISTERED.get_or_init(|| unsafe {
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(source_proc),
                    hInstance: HINSTANCE(GetModuleHandleW(None).unwrap().0),
                    lpszClassName: w!("Otter.Test.PopupFamily"),
                    hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as usize as *mut _),
                    ..Default::default()
                }),
                0
            );
        });
        let window = Self(unsafe {
            CreateWindowExW(
                style,
                w!("Otter.Test.PopupFamily"),
                w!("Otter owned popup lifecycle test"),
                WS_POPUP,
                screen.left,
                screen.top,
                screen.right - screen.left,
                screen.bottom - screen.top,
                owner,
                None,
                Some(HINSTANCE(GetModuleHandleW(None).unwrap().0)),
                None,
            )
            .unwrap()
        });
        if owner.is_some() {
            unsafe {
                SetWindowLongPtrW(window.0, GWLP_USERDATA, 1);
            }
        }
        window
    }
    fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.0, SW_SHOWNOACTIVATE);
        }
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                DestroyWindow(self.0).unwrap();
            }
        }
    }
}
fn pump_until(session: &mut Session, ready: impl Fn(&Session) -> bool) {
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        assert!(
            session.poll().is_none(),
            "root session ended during popup test"
        );
        if ready(session) {
            return;
        }
        assert!(Instant::now() < until, "popup lifecycle deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "owned visible windows; helper filtering before popup capacity and resize rediscovery"]
fn tiny_helpers_do_not_exhaust_popup_slots_and_can_grow_into_popups() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    // 候选使用物理 frame；DPI-unaware 的 1 逻辑像素在缩放桌面可能大于 1 物理像素。
    let old_dpi =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let source = Source::new(None, rect(100, 100, 200, 200), false);
    let popup = Source::new(Some(source.0), rect(150, 160, 100, 80), true);
    source.show();
    popup.show();
    let helpers: Vec<_> = (0..LIMIT + 1)
        .map(|_| {
            let helper = Source::new(Some(source.0), rect(100, 100, 1, 1), true);
            helper.show();
            helper
        })
        .collect();
    let root = Target::bind(source.0).unwrap();
    for helper in &helpers {
        assert!(OwnedPopup::bind(&root, helper.0).is_ok());
    }
    let discovered = || {
        discover(&root, None, &[], &[])
            .unwrap()
            .into_iter()
            .map(|scope| scope.target().unwrap().hwnd())
            .collect::<Vec<_>>()
    };
    assert_eq!(discovered(), [popup.0]);
    unsafe {
        SetWindowPos(
            helpers[0].0,
            None,
            0,
            0,
            90,
            70,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    let grown = discovered();
    assert_eq!(grown.len(), 2);
    assert!(grown.contains(&helpers[0].0));
    assert!(grown.contains(&popup.0));
    unsafe {
        SetWindowPos(
            helpers[0].0,
            None,
            0,
            0,
            1,
            1,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    assert_eq!(discovered(), [popup.0]);
    unsafe {
        SetThreadDpiAwarenessContext(old_dpi);
    }
}

#[path = "composition_live_popup_parent_tests.rs"]
mod parent_tests;
#[path = "composition_live_popup_rebind_tests.rs"]
mod rebind_tests;
#[path = "composition_live_popup_retry_tests.rs"]
mod retry_tests;
#[path = "composition_live_popup_thread_tests.rs"]
mod thread_tests;

#[test]
fn discovery_keeps_existing_popups_beyond_the_session_admission_limit() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None, rect(100, 100, 200, 200), false);
    let target = Target::bind(root.0).unwrap();
    let popups: Vec<_> = (0..LIMIT + 2)
        .map(|_| Source::new(Some(root.0), rect(150, 150, 80, 60), true))
        .collect();
    for popup in &popups {
        popup.show();
    }
    let visible: Vec<_> = discover(&target, None, &[], &[])
        .unwrap()
        .into_iter()
        .map(|scope| scope.target().unwrap().hwnd())
        .collect();
    for popup in &popups {
        assert!(
            visible.contains(&popup.0),
            "discovery is not session admission"
        );
    }
    unsafe {
        let _ = ShowWindow(popups[0].0, SW_HIDE);
    }
    assert!(!discover(&target, None, &[], &[])
        .unwrap()
        .iter()
        .any(|scope| scope.target().unwrap().hwnd() == popups[0].0));
}

#[test]
#[ignore = "interactive Windows desktop and WGC; owned source/popup only"]
fn owned_popup_mirrors_without_activation_and_reaps_on_hide_and_parent_close() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let old_dpi =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let source = Source::new(None, rect(100, 100, 200, 200), false);
    let popup = Source::new(Some(source.0), rect(150, 160, 100, 80), true);
    source.show();
    let target = Target::bind(source.0).unwrap();
    let mut session = Session::start_window(target, rect(600, 100, 200, 200)).unwrap();
    session.show().unwrap();
    popup.show();
    pump_until(&mut session, |s| {
        s.popups.members.len() == 1
            && s.popups.members[0]
                .session
                .window
                .as_ref()
                .unwrap()
                .frame_for_test()
                .is_some()
    });
    let member = &session.popups.members[0];
    let mirror = member.session.window.as_ref().unwrap().popup_hwnd();
    assert_eq!(member.hwnd, popup.0 .0 as usize);
    assert_eq!(member.placement, rect(650, 160, 100, 80));
    pump_until(&mut session, |s| {
        s.popups.members[0].session.input_enabled()
    });
    let before = CLICKS.with(|clicks| clicks.get());
    unsafe {
        let capture = windows::Win32::UI::Input::KeyboardAndMouse::GetCapture();
        SendMessageW(
            mirror,
            WM_LBUTTONDOWN,
            Some(WPARAM(1)),
            Some(LPARAM(20 | (20 << 16))),
        );
        assert_eq!(
            windows::Win32::UI::Input::KeyboardAndMouse::GetCapture(),
            capture
        );
        SendMessageW(
            mirror,
            WM_LBUTTONUP,
            Some(WPARAM(0)),
            Some(LPARAM(20 | (20 << 16))),
        );
    }
    pump_until(&mut session, |_| {
        CLICKS.with(|clicks| clicks.get().1 == before.1 + 1)
    });
    assert_eq!(
        CLICKS.with(|clicks| clicks.get()),
        (before.0 + 1, before.1 + 1)
    );
    unsafe {
        // 主 B 没有附属 owner；直接调整其无边框客户区，再观察子镜像的生产跟随。
        SetWindowPos(
            session.window.as_ref().unwrap().popup_hwnd(),
            None,
            800,
            100,
            400,
            400,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
    }
    pump_until(&mut session, |s| {
        s.popups.members[0].placement == rect(900, 220, 200, 160)
    });
    session.window.as_ref().unwrap().suspend_source_input();
    pump_until(&mut session, |s| {
        !s.popups.members[0]
            .session
            .window
            .as_ref()
            .unwrap()
            .popup_control_allowed()
    });
    session.window.as_ref().unwrap().request_default_input();
    pump_until(&mut session, |s| {
        s.popups.members[0].session.input_enabled()
    });
    unsafe {
        assert!(
            IsWindowVisible(popup.0).as_bool(),
            "A popup must remain visible"
        );
        assert_eq!(
            GetWindow(mirror, GW_OWNER).unwrap(),
            session.window.as_ref().unwrap().popup_hwnd()
        );
        assert_ne!(
            GetWindowLongPtrW(mirror, GWL_EXSTYLE) & WS_EX_NOACTIVATE.0 as isize,
            0
        );
        assert_eq!(
            SendMessageW(mirror, WM_MOUSEACTIVATE, None, None).0,
            MA_NOACTIVATE as isize
        );
        let _ = ShowWindow(popup.0, SW_HIDE);
    }
    pump_until(&mut session, |s| s.popups.is_empty());
    assert!(!unsafe { IsWindow(Some(mirror)) }.as_bool());
    popup.show();
    pump_until(&mut session, |s| s.popups.members.len() == 1);
    let mirror = session.popups.members[0]
        .session
        .window
        .as_ref()
        .unwrap()
        .popup_hwnd();
    session.stop();
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(result) = session.poll() {
            result.unwrap();
            break;
        }
        assert!(
            Instant::now() < until,
            "parent close must reap popup capture"
        );
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!unsafe { IsWindow(Some(mirror)) }.as_bool());
    assert!(unsafe { IsWindowVisible(popup.0) }.as_bool());
    unsafe {
        SetThreadDpiAwarenessContext(old_dpi);
    }
}
