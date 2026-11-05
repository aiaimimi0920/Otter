//! 显式运行的系统触摸探针：只向自有可见 HWND 注入，记录真实消息，不伪造 pointer 数据。
use super::*;
use std::time::{Duration, Instant};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::Pointer::*;
#[path = "surface_slider_gesture_probe_tests.rs"]
mod gesture_probe;

struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
unsafe fn parent() -> Parent {
    // 系统输入夹具使用独立顶层类，不依赖 STATIC 的默认命中行为。
    static REGISTER: std::sync::Once = std::sync::Once::new();
    REGISTER.call_once(|| {
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(parent_procedure),
                lpszClassName: w!("Otter.Test.SliderTouch"),
                ..Default::default()
            }),
            0
        );
    });
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("Otter.Test.SliderTouch"),
        w!("Otter isolated touch probe"),
        WS_OVERLAPPEDWINDOW,
        400,
        300,
        300,
        250,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let mut log = Box::<Log>::default();
    assert!(SetWindowSubclass(hwnd, Some(record), 1, (&mut *log as *mut Log) as usize).as_bool());
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    Parent { hwnd, log }
}
unsafe extern "system" fn parent_procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, message, wp, lp)
}

#[derive(Default, Debug)]
struct Trace {
    messages: Vec<(u32, u32)>,
    pointers: usize,
    overflow: bool,
    gestures: Vec<(u32, u32, usize)>,
    wheels: Vec<(u32, i16, usize)>,
}
unsafe extern "system" fn trace(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let mut detail = 0;
    if matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
        let log = &mut *(data as *mut Trace);
        if log.wheels.len() < 128 {
            log.wheels.push((
                message,
                (wp.0 >> 16) as u16 as i16,
                GetMessageExtraInfo().0 as usize,
            ));
        } else {
            log.overflow = true;
        }
    }
    if message == WM_GESTURE {
        use windows::Win32::UI::Input::Touch::*;
        let mut info = GESTUREINFO {
            cbSize: std::mem::size_of::<GESTUREINFO>() as u32,
            ..Default::default()
        };
        if GetGestureInfo(HGESTUREINFO(lp.0 as *mut _), &mut info).is_ok() {
            let log = &mut *(data as *mut Trace);
            if log.gestures.len() < 128 {
                log.gestures
                    .push((info.dwID, info.dwFlags, info.hwndTarget.0 as usize));
            } else {
                log.overflow = true;
            }
        }
    }
    if matches!(message, WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP) {
        let mut info = POINTER_INFO::default();
        if GetPointerInfo((wp.0 & 0xffff) as u32, &mut info).is_ok()
            && info.pointerType == PT_TOUCH
            && info.hwndTarget == hwnd
        {
            (*(data as *mut Trace)).pointers += 1;
            detail = info.pointerFlags.0;
        }
    }
    if matches!(
        message,
        WM_POINTERDOWN
            | WM_POINTERUPDATE
            | WM_POINTERUP
            | WM_POINTERCAPTURECHANGED
            | WM_GESTURE
            | WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_MOUSEMOVE
            | CLICK_BEGIN
            | CLICK_CANCEL
            | CLICK_END
            | WM_ACTIVATE
            | WM_CANCELMODE
            | WM_KILLFOCUS
            | WM_SIZE
            | WM_DPICHANGED
    ) {
        if matches!(
            message,
            WM_ACTIVATE | WM_CANCELMODE | WM_KILLFOCUS | WM_SIZE | WM_DPICHANGED
        ) {
            detail = wp.0 as u32;
        }
        let log = &mut *(data as *mut Trace);
        if log.messages.len() < 128 {
            log.messages.push((message, detail));
        } else {
            log.overflow = true;
        }
    }
    // 观察器不消费消息：生产子类与系统默认处理保持原样。
    DefSubclassProc(hwnd, message, wp, lp)
}
struct Observer {
    hwnd: HWND,
    log: Box<Trace>,
}
impl Observer {
    unsafe fn new(hwnd: HWND) -> Self {
        let mut log = Box::<Trace>::default();
        assert!(SetWindowSubclass(
            hwnd,
            Some(trace),
            0x54505242,
            (&mut *log as *mut Trace) as usize
        )
        .as_bool());
        Self { hwnd, log }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.hwnd)).as_bool() {
                let _ = RemoveWindowSubclass(self.hwnd, Some(trace), 0x54505242);
            }
        }
    }
}
fn pump(milliseconds: u64) {
    let until = Instant::now() + Duration::from_millis(milliseconds);
    while Instant::now() < until {
        unsafe {
            let mut message = MSG::default();
            // 每轮有界，避免持续消息流吞掉截止时间。
            for _ in 0..128 {
                if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
struct Contact {
    info: POINTER_TOUCH_INFO,
    active: bool,
}
impl Contact {
    unsafe fn new(hwnd: HWND, point: POINT) -> Self {
        // 不抢占其他应用；若自有子窗口没有真实命中，拒绝全局注入。
        let hit = WindowFromPoint(point);
        let parent = GetParent(hwnd).unwrap();
        let mut bounds = RECT::default();
        GetWindowRect(parent, &mut bounds).unwrap();
        println!("TOUCH_TARGET parent={parent:?} child={hwnd:?} point={point:?} bounds={bounds:?} visible={} enabled={} exstyle={:x}",
            IsWindowVisible(parent).as_bool(), IsWindowEnabled(parent).as_bool(), GetWindowLongPtrW(parent, GWL_EXSTYLE));
        assert_eq!(
            GetAncestor(hit, GA_ROOT),
            parent,
            "touch target is occluded: hit={hit:?}, child={hwnd:?}"
        );
        let mut local = point;
        assert!(ScreenToClient(parent, &mut local).as_bool());
        assert_eq!(
            ChildWindowFromPointEx(parent, local, CWP_SKIPINVISIBLE | CWP_SKIPDISABLED),
            hwnd
        );
        Self {
            info: POINTER_TOUCH_INFO {
                pointerInfo: POINTER_INFO {
                    pointerType: PT_TOUCH,
                    pointerId: 0,
                    ptPixelLocation: point,
                    ..Default::default()
                },
                ..Default::default()
            },
            active: false,
        }
    }
    unsafe fn send(&mut self, point: POINT, flags: POINTER_FLAGS) {
        self.info.pointerInfo.ptPixelLocation = point;
        self.info.pointerInfo.pointerFlags = flags;
        InjectTouchInput(&[self.info]).expect("system touch injection failed");
        self.active = flags.0 & POINTER_FLAG_UP.0 == 0;
        pump(35);
    }
}
impl Drop for Contact {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                // panic/失败也取消本探针的接触，不留下悬挂按压。
                self.info.pointerInfo.pointerFlags = POINTER_FLAG_UP | POINTER_FLAG_CANCELED;
                let _ = InjectTouchInput(&[self.info]);
            }
            pump(60);
        }
    }
}

#[test]
#[ignore = "explicit interactive desktop probe; injects touch only into owned visible Slider"]
fn surface_slider_system_touch_probe() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_TOUCH_PROBE").as_deref(),
        Ok("1")
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    unsafe { InitializeTouchInjection(1, TOUCH_FEEDBACK_NONE).unwrap() };
    for mode in ["tap", "horizontal", "vertical", "cancel"] {
        let parent = unsafe { parent() };
        let control = control(&parent);
        unsafe {
            let _ = SetForegroundWindow(parent.hwnd);
            SetWindowPos(
                parent.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            )
            .unwrap();
        }
        let hwnd = control.hwnd().unwrap();
        let observer = unsafe { Observer::new(hwnd) };
        let parent_observer = unsafe { Observer::new(parent.hwnd) };
        let mut start = POINT { x: 100, y: 15 };
        unsafe { assert!(ClientToScreen(hwnd, &mut start).as_bool()) };
        pump(100);
        let mut contact = unsafe { Contact::new(hwnd, start) };
        unsafe {
            contact.send(
                start,
                POINTER_FLAG_DOWN | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
            );
            let mut end = start;
            if mode != "tap" {
                for step in 1..=5 {
                    end = POINT {
                        x: start.x + if mode == "vertical" { 0 } else { step * 12 },
                        y: start.y + if mode == "vertical" { step * 12 } else { 0 },
                    };
                    contact.send(
                        end,
                        POINTER_FLAG_UPDATE | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
                    );
                }
            }
            contact.send(
                end,
                POINTER_FLAG_UP
                    | if mode == "cancel" {
                        POINTER_FLAG_CANCELED
                    } else {
                        POINTER_FLAG_NONE
                    },
            );
        }
        pump(400);
        println!(
            "TOUCH_PROBE {mode}: value={} events={:?} trace={:?}",
            control.value().unwrap(),
            parent.log.events,
            observer.log
        );
        println!("TOUCH_PARENT {mode}: {:?}", parent_observer.log);
        assert!(!observer.log.overflow);
        assert!(!parent_observer.log.overflow);
        unsafe { assert_ne!(GetCapture(), hwnd, "completed touch retained mouse capture") };
        if matches!(mode, "horizontal" | "cancel") {
            assert!(control.value().unwrap() > 50.);
            assert!(parent.log.events.iter().any(|(phase, _)| *phase == 0));
            assert_eq!(
                parent
                    .log
                    .events
                    .iter()
                    .filter(|(phase, _)| *phase == 1)
                    .count(),
                usize::from(mode == "horizontal")
            );
        } else {
            assert_eq!(control.value().unwrap(), 50.);
            assert!(parent.log.events.is_empty());
        }
        assert!(
            parent_observer
                .log
                .messages
                .iter()
                .all(|(message, _)| !matches!(*message, CLICK_BEGIN | CLICK_END)),
            "touch must not arm or finish a mouse click"
        );
        assert!(
            observer.log.pointers > 0,
            "no real system pointer reached the owned Slider"
        );
    }
}

#[test]
fn surface_slider_touch_promotion_distinguishes_mouse_pen_and_cursor_ids() {
    for id in 0..128 {
        assert!(messages::is_touch_promotion(0xff515780 | id));
        assert!(!messages::is_touch_promotion(0xff515700 | id));
    }
    for other in [0, 0x80, 0xff515680, 0xff515880, usize::MAX] {
        assert!(!messages::is_touch_promotion(other));
    }
}

struct Extra(LPARAM);
impl Drop for Extra {
    fn drop(&mut self) {
        unsafe {
            SetMessageExtraInfo(self.0);
        }
    }
}
unsafe fn origin_mouse(hwnd: HWND, message: u32, x: i32, touch: bool) {
    // 仅修改本测试线程消息元数据；真实系统来源另由显式注入测试证明。
    let _extra = Extra(SetMessageExtraInfo(LPARAM(if touch {
        0xff515780
    } else {
        0
    })));
    mouse(hwnd, message, x);
}

#[test]
fn surface_slider_drag_rejects_other_origin_moves_releases_and_restarts() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for touch in [false, true] {
        let parent = Parent::new();
        let control = control(&parent);
        let hwnd = control.hwnd().unwrap();
        let observer = unsafe { Observer::new(parent.hwnd) };
        unsafe {
            let (left, right, _, _) = paint::track(hwnd).unwrap();
            origin_mouse(hwnd, WM_LBUTTONDOWN, left, touch);
            assert_eq!(control.value().unwrap(), 0.);
            origin_mouse(hwnd, WM_MOUSEMOVE, right, !touch);
            origin_mouse(hwnd, WM_LBUTTONUP, right, !touch);
            origin_mouse(hwnd, WM_LBUTTONDOWN, right, !touch);
            assert_eq!(control.value().unwrap(), 0.);
            assert_eq!(GetCapture(), hwnd);
            origin_mouse(hwnd, WM_MOUSEMOVE, right, touch);
            origin_mouse(hwnd, WM_LBUTTONUP, right, touch);
            assert_ne!(GetCapture(), hwnd);
        }
        assert_eq!(parent.log.events, [(0, 0.), (0, 100.), (1, 100.)]);
        let clicks: Vec<_> = observer
            .log
            .messages
            .iter()
            .map(|(message, _)| *message)
            .filter(|message| matches!(*message, CLICK_BEGIN | CLICK_END))
            .collect();
        assert_eq!(
            clicks,
            if touch {
                vec![]
            } else {
                vec![CLICK_BEGIN, CLICK_END]
            }
        );
    }
}
