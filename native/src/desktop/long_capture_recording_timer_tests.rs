//! 私有 timer 注入只替换录制的重新设时；真实 waiting timer、模态关闭与 worker 取消仍走产品路径。
use super::super::super as dialog;
use super::*;
use crate::desktop::{capture_input::finish_route, Window};
use std::rc::Rc;
use windows::{core::w, Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled};

struct Context {
    input: Input,
    capture: Rc<Cell<bool>>,
    inflight: bool,
    calls: Arc<AtomicUsize>,
    hwnd: Cell<HWND>,
    injected: Cell<bool>,
    failure: RefCell<Option<String>>,
}
unsafe fn inject(hwnd: HWND, context: &Context) {
    let input = &context.input;
    context.hwnd.set(hwnd);
    SetWindowPos(
        hwnd,
        None,
        -32000,
        -32000,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
    let owner = GetWindow(hwnd, GW_OWNER).unwrap();
    assert!(!IsWindowEnabled(owner).as_bool());
    *input.recording.hook.borrow_mut() = Some(hook::Hook::install(hwnd, owner.0 as isize).unwrap());
    hook::test_notify(POINT::default(), owner.0 as isize);
    assert!(!hook::test_inactive());
    if context.inflight {
        input
            .recording
            .schedule
            .borrow_mut()
            .notify(Instant::now() - Duration::from_millis(25));
    }
    drive_with_timer(hwnd, input, |delay| {
        context.injected.set(true);
        assert_eq!(input.job.borrow().is_pending(), context.inflight);
        if context.inflight {
            // 此时 waiting 已成功设立真实 timer 且禁用预览；只让随后的重设失败。
            assert_eq!(delay, 24);
            assert!(!IsWindowEnabled(GetDlgItem(Some(hwnd), 102).unwrap()).as_bool());
            let deadline = Instant::now() + Duration::from_secs(2);
            while context.calls.load(Ordering::Acquire) == 0 {
                assert!(Instant::now() < deadline, "worker never entered capture");
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        false
    });
    assert_eq!(
        input.error.borrow().as_deref(),
        Some("无法安排长截图录制等待，录制已安全关闭。")
    );
    assert_eq!(input.recording.schedule.borrow().phase(), Phase::Cancelled);
    assert!(input.recording.hook.borrow().is_none() && hook::test_inactive());
    assert!(!input.job.borrow().is_pending());
    assert_eq!(
        input.job.borrow_mut().capture(),
        Err(crate::long_capture::DispatchError::Stopped)
    );
    assert!(
        KillTimer(Some(hwnd), TIMER).is_err(),
        "waiting timer survived failure"
    );
    assert!(
        !finish_route::request(&context.capture),
        "finish route survived failure"
    );
    // 已排队的旧 wheel 通知即使在 EndDialog 生效前送达，也不能重启调度或任务。
    let mut message = MSG::default();
    assert!(PeekMessageW(&mut message, Some(hwnd), MESSAGE, MESSAGE, PM_REMOVE).as_bool());
    dialog::handle(hwnd, message.message, message.wParam, message.lParam);
    assert_eq!(input.recording.schedule.borrow().phase(), Phase::Cancelled);
    assert!(!input.job.borrow().is_pending());
    assert!(input.output.borrow().is_none());
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message != WM_INITDIALOG {
        return dialog::procedure(hwnd, message, wp, lp);
    }
    let context = &*(lp.0 as *const Context);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dialog::procedure(
            hwnd,
            message,
            wp,
            LPARAM(&context.input as *const Input as isize),
        );
        inject(hwnd, context);
    }));
    if result.is_err() {
        *context.failure.borrow_mut() = Some("timer failure injection assertion failed".into());
        dialog::close(hwnd, &context.input);
    }
    1
}
unsafe fn run(inflight: bool) {
    let owner = Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("capture timer fault owner"),
            WS_OVERLAPPED,
            -32000,
            -32000,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    );
    let (release, gate) = mpsc::channel();
    let (mut input, calls) = setup(vec![Ok(frame(0, 4))], Some(gate));
    let capture = Rc::new(Cell::new(true));
    input.finish = dialog::finish::State::new(capture.clone());
    let context = Context {
        input,
        capture,
        inflight,
        calls,
        hwnd: Cell::new(HWND::default()),
        injected: Cell::new(false),
        failure: RefCell::new(None),
    };
    let template = dialog::layout::template();
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner.0),
        Some(procedure),
        LPARAM(&context as *const Context as isize),
    );
    let _ = release.send(());
    assert!(
        context.failure.borrow().is_none(),
        "{:?}",
        context.failure.borrow()
    );
    assert!(context.injected.get());
    assert_eq!(
        result, 0,
        "timer failure must not return a successful capture"
    );
    assert!(
        !IsWindow(Some(context.hwnd.get())).as_bool(),
        "disabled dialog survived"
    );
    assert!(
        IsWindowEnabled(owner.0).as_bool(),
        "modal owner stayed disabled"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !context.input.job.borrow().is_finished() {
        assert!(Instant::now() < deadline, "cancelled worker leaked");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(context.input.job.borrow_mut().take_ready().is_none());
    assert!(context.input.output.borrow().is_none());
    assert_eq!(context.input.job.borrow().snapshot().frame_count, 1);
    assert_eq!(context.calls.load(Ordering::Acquire), usize::from(inflight));
    assert!(hook::test_inactive());
    drop(context); // SessionJob::drop 仍负责 join，取消后的迟到帧没有发布通道。
    let fresh = manual_input();
    let window = hidden(&fresh);
    command(window.0, 1);
    dialog::tests::settle(window.0, &fresh);
    assert!(
        fresh.output.borrow().is_some(),
        "new session was poisoned by old failure"
    );
}
#[test]
fn timer_failure_without_worker_closes_modal_and_retires_listener_and_finish_route() {
    unsafe { run(false) }
}
#[test]
fn timer_failure_after_waiting_cancels_inflight_reply_and_destroys_disabled_modal() {
    unsafe { run(true) }
}
