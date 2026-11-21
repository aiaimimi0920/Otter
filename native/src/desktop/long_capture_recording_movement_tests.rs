//! 真实 BUTTON 点击在采样忙态仍排队；位移串行提交，停止/完成/取消丢弃未执行意图。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;

unsafe fn block_capture(hwnd: HWND, input: &Input) {
    input
        .recording
        .schedule
        .borrow_mut()
        .notify(Instant::now() - Duration::from_millis(25));
    drive(hwnd, input);
    assert!(input.job.borrow().is_pending());
}
unsafe fn click(hwnd: HWND, id: i32) {
    let button = GetDlgItem(Some(hwnd), id).unwrap();
    assert!(
        IsWindowEnabled(button).as_bool(),
        "sampling must not disable extension arrows"
    );
    SendMessageW(button, BM_CLICK, None, None);
}
#[test]
fn extension_clicks_during_capture_are_kept_and_run_before_the_next_periodic_sample() {
    unsafe {
        let (release, gate) = mpsc::channel();
        let (input, calls) = setup(
            vec![Ok(frame(0, 0)), Ok(frame(1, 0)), Ok(frame(2, 0))],
            Some(gate),
        );
        let window = hidden(&input);
        block_capture(window.0, &input);
        click(window.0, 110);
        click(window.0, 110);
        assert!(input.movement.queued());
        assert_eq!(input.movement.region().left, 0);
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while input.movement.region().left != 2 {
            super::super::super::handle(window.0, WM_TIMER, WPARAM(TIMER), LPARAM(0));
            assert!(input.recording.running());
            assert!(Instant::now() < deadline, "queued arrow did not commit");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(!input.movement.queued());
        assert_eq!(calls.load(Ordering::Acquire), 3);
        assert_eq!(input.job.borrow().snapshot().spatial_position, Some((2, 0)));
        command(window.0, 112);
        pump(window.0, &input);
        command(window.0, 1);
        pump(window.0, &input);
        assert_eq!(
            input
                .output
                .borrow()
                .as_ref()
                .map(|image| (image.width, image.height)),
            Some((34, 48))
        );
    }
}
#[test]
fn queued_extensions_are_bounded_and_retired_by_stop_finish_cancel_and_capture_error() {
    unsafe {
        for end in [112, 1, 2, 0] {
            let (release, gate) = mpsc::channel();
            let frames = if end == 0 {
                vec![Err("capture fixture failure".into())]
            } else {
                vec![Ok(frame(0, 4))]
            };
            let (input, _) = setup(frames, Some(gate));
            let window = hidden(&input);
            block_capture(window.0, &input);
            for _ in 0..16 {
                click(window.0, 110);
            }
            assert!(!IsWindowEnabled(GetDlgItem(Some(window.0), 110).unwrap()).as_bool());
            command(window.0, 110); // 原始消息也不能突破队列上限。
            if end != 0 {
                command(window.0, end);
            }
            release.send(()).unwrap();
            if end != 2 {
                pump(window.0, &input);
            }
            assert!(!input.movement.queued());
            assert_eq!(input.movement.region().left, 0);
        }
    }
}
