//! Completion-notification regressions use only hidden HWNDs and synthetic pixels; no desktop reads or system input injection.
use super::*;
thread_local! { static TICKS: Cell<usize> = const { Cell::new(0) }; }
unsafe extern "system" fn count_timer(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lp.0 != 0 {
        let message = &*(lp.0 as *const MSG);
        if wp.0 == PM_REMOVE.0 as usize && message.message == WM_TIMER && message.wParam.0 == TIMER
        {
            TICKS.set(TICKS.get() + 1);
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
#[test]
fn quiet_completion_uses_the_worker_handle_without_timer_polling() {
    unsafe {
        let _probe = ShowProbe(
            SetWindowsHookExW(
                WH_GETMESSAGE,
                Some(count_timer),
                None,
                windows::Win32::System::Threading::GetCurrentThreadId(),
            )
            .unwrap(),
        );
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("completion owner"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let mut input = Input::new(
            Box::new(|| {
                CaptureJob::spawn(|_| {
                    std::thread::sleep(Duration::from_millis(5));
                    Ok(outcome())
                })
            }),
            false,
        );
        input.quiet = true;
        TICKS.set(0);
        let start = Instant::now();
        assert!(run(owner.0, input).unwrap().is_some());
        eprintln!(
            "capture-quiet-ready: elapsed_us={} timer_ticks={}",
            start.elapsed().as_micros(),
            TICKS.get()
        );
        assert_eq!(TICKS.get(), 0, "Quiet capture must not wait for WM_TIMER");
    }
}
