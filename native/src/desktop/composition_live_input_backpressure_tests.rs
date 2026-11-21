//! 慢源仍保留最新指针和有序边沿；使用自有 HWND、真实输入 worker 与可控发送闸门。
use super::*;
use crate::{
    desktop::Apartment,
    live_input::{Backend, Event},
    live_input_worker::{Command, Worker},
};
use std::{
    sync::{atomic::AtomicUsize, mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use windows::core::Interface;
use windows::Win32::{
    Foundation::{HWND, LPARAM, POINT, WPARAM},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::*,
};

#[derive(Default)]
struct Trace {
    sent: Vec<Event>,
    released: Vec<Event>,
    fail: Option<crate::live_window_target::input_backend::Error>,
}
struct SlowSource {
    trace: Arc<Mutex<Trace>>,
    entered: mpsc::SyncSender<()>,
    gate: Option<mpsc::Receiver<()>>,
    release_gate: Option<mpsc::Receiver<()>>,
}
impl Backend for SlowSource {
    type Target = u8;
    type Error = crate::live_window_target::input_backend::Error;
    fn preflight(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn resolve(&mut self, _: Event, _: Option<u8>) -> Result<u8, Self::Error> {
        Ok(1)
    }
    fn send(&mut self, _: u8, event: Event, _: u8, _: u32) -> Result<(), Self::Error> {
        if let Some(error) = self.trace.lock().unwrap().fail.take() {
            return Err(error);
        }
        if matches!(event, Event::Move { .. }) {
            if let Some(gate) = self.gate.take() {
                self.entered.send(()).unwrap();
                gate.recv_timeout(Duration::from_secs(5)).unwrap();
            }
        }
        self.trace.lock().unwrap().sent.push(event);
        Ok(())
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        Some(target)
    }
    fn release(&mut self, _: u8, event: Event, _: u8) -> Result<(), Self::Error> {
        if let Some(gate) = self.release_gate.take() {
            self.entered.send(()).unwrap();
            gate.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        self.trace.lock().unwrap().released.push(event);
        Ok(())
    }
}
#[test]
fn browser_query_transition_preserves_resume_but_unknown_action_fails() {
    use crate::live_window_target::input_backend::Error;
    let _apartment = Apartment::new().unwrap();
    for transient in [true, false] {
        let mut fixture = Fixture::with_release_gate(true);
        fixture
            .session
            .window
            .as_ref()
            .unwrap()
            .request_default_input();
        fixture.trace.lock().unwrap().fail = Some(if transient {
            Error::BrowserTransition("0x80131505 before action".into())
        } else {
            Error::BrowserScroll("0x80131505 after action".into())
        });
        unsafe {
            SendMessageW(
                fixture.hwnd(),
                WM_MOUSEMOVE,
                None,
                Some(LPARAM(64 | (64 << 16))),
            );
        }
        fixture.session.process_window_input();
        // 模拟后端没有真实源绑定；仅结算回复，不替它执行下一轮真实 Backend 准入。
        let deadline = Instant::now() + Duration::from_secs(3);
        while fixture.session.input.is_some() {
            fixture.session.poll_input();
            assert!(Instant::now() < deadline, "input worker did not settle");
            std::thread::yield_now();
        }
        assert_eq!(
            fixture
                .session
                .window
                .as_ref()
                .unwrap()
                .input_resume_pending(),
            transient
        );
        assert_eq!(fixture.session.take_input_error().is_none(), transient);
        assert_eq!(fixture.session.take_input_notice().is_none(), transient);
        assert_eq!(fixture.session.input_cleanup().unwrap().failed, 0);
        assert!(
            fixture.trace.lock().unwrap().sent.is_empty(),
            "failed input must not replay"
        );
        // 只检查新鼠标输入的恢复意图，不替自有模拟后端伪造真实源绑定。
        unsafe {
            SendMessageW(
                fixture.hwnd(),
                WM_MOUSEMOVE,
                None,
                Some(LPARAM(64 | (64 << 16))),
            );
        }
        assert_eq!(
            fixture
                .session
                .window
                .as_ref()
                .unwrap()
                .source_activation_requested(),
            transient
        );
    }
}
#[test]
fn browser_resume_does_not_override_cancel_or_remote_owner() {
    let _apartment = Apartment::new().unwrap();
    let fixture = Fixture::with_release_gate(true);
    let window = fixture.session.window.as_ref().unwrap();
    window.request_default_input();
    window.suspend_source_input();
    window.resume_browser_input();
    assert!(window.input_resume_pending());
    window.block_local_input(true);
    window.suspend_source_input();
    window.resume_browser_input();
    assert!(!window.input_resume_pending());
    window.block_local_input(false);
    unsafe {
        SendMessageW(
            fixture.hwnd(),
            WM_SYSCOMMAND,
            Some(windows::Win32::Foundation::WPARAM(0x1ff0)),
            None,
        );
    }
    window.resume_browser_input();
    assert!(!window.input_resume_pending());
}
struct Fixture {
    session: Session,
    trace: Arc<Mutex<Trace>>,
    entered: mpsc::Receiver<()>,
    release: mpsc::SyncSender<()>,
}
impl Fixture {
    fn new() -> Self {
        Self::with_release_gate(false)
    }
    fn with_release_gate(block_release: bool) -> Self {
        let active = Arc::new(AtomicUsize::new(0));
        let (ready, source) = mpsc::sync_channel(1);
        let (job, window) = LiveJob::<Texture>::spawn_bound(
            &active,
            |consumer| LiveWindow::new(consumer, 128, 128).map_err(io::Error::other),
            move |source, _| {
                let (wake, wait) = mpsc::channel();
                source
                    .on_idle(Arc::new(move || {
                        let _ = wake.send(());
                    }))
                    .unwrap();
                ready.send(source.clone()).unwrap();
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                Ok(())
            },
        )
        .unwrap();
        let source = source.recv_timeout(Duration::from_secs(2)).unwrap();
        unsafe {
            let device = crate::desktop::composition::create_device(
                windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP,
            )
            .unwrap();
            let protection: windows::Win32::Graphics::Direct3D11::ID3D11Multithread =
                device.GetImmediateContext().unwrap().cast().unwrap();
            let _ = protection.SetMultithreadProtected(true);
            let frame = super::super::tests::frame(&device, [0, 255, 0, 255]);
            source
                .publish(Arc::try_unwrap(frame).ok().unwrap().resource)
                .unwrap();
            window.allow_input().unwrap();
            SendMessageW(window.hwnd_for_test(), WM_PAINT, None, None);
        }
        let trace = Arc::new(Mutex::new(Trace::default()));
        let (entered, entered_rx) = mpsc::sync_channel(1);
        let (release, gate) = mpsc::sync_channel(1);
        let (gate, release_gate) = if block_release {
            (None, Some(gate))
        } else {
            (Some(gate), None)
        };
        let backend = SlowSource {
            trace: trace.clone(),
            entered,
            gate,
            release_gate,
        };
        let mut worker = Worker::start_for_ui(move || Ok(backend)).unwrap();
        worker.submit(Command::Enable).unwrap();
        let mut session = Session::new(job, window);
        session.input = Some(worker);
        poll_until(&mut session, |session| session.input_enabled());
        Self {
            session,
            trace,
            entered: entered_rx,
            release,
        }
    }
    fn hwnd(&self) -> HWND {
        self.session.window.as_ref().unwrap().hwnd_for_test()
    }
    fn message(&mut self, message: u32, wp: usize, lp: isize) {
        unsafe {
            SendMessageW(self.hwnd(), message, Some(WPARAM(wp)), Some(LPARAM(lp)));
        }
        let result = self.session.poll();
        assert!(result.is_none(), "Live fixture ended: {result:?}");
    }
    fn motion(&mut self, x: u16) {
        self.message(WM_MOUSEMOVE, 0, (u32::from(x) | (64 << 16)) as isize);
    }
    fn block_motion(&mut self) {
        self.motion(16);
        self.entered.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 即使断言失败，也先解除自有后端闸门，再由 Session/Worker 回收线程。
        let _ = self.release.try_send(());
    }
}
fn poll_until(session: &mut Session, done: impl Fn(&Session) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let result = session.poll();
        assert!(result.is_none(), "Live fixture ended: {result:?}");
        if done(session) {
            return;
        }
        assert!(Instant::now() < deadline, "input worker did not settle");
        std::thread::yield_now();
    }
}

#[test]
fn slow_source_coalesces_motion_across_polls_and_preserves_edges() {
    let _apartment = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    fixture.block_motion();
    for _ in 0..4 {
        for x in 20..100 {
            fixture.motion(x);
        }
    }
    fixture.motion(80);
    fixture.message(WM_KEYDOWN, 65, 0);
    let mut point = POINT { x: 64, y: 64 };
    unsafe {
        assert!(ClientToScreen(fixture.hwnd(), &mut point).as_bool());
    }
    let wheel =
        (u32::from(point.x as i16 as u16) | (u32::from(point.y as i16 as u16) << 16)) as isize;
    fixture.message(WM_MOUSEWHEEL, 120 << 16, wheel);
    fixture.message(WM_KEYUP, 65, 0);
    fixture.motion(96);
    let enabled = fixture.session.input_enabled();
    let error = fixture.session.take_input_error();
    let outstanding = fixture.session.input.as_ref().map(Worker::outstanding);
    fixture.release.send(()).unwrap();
    assert!(
        enabled,
        "ordinary pointer backlog revoked source control: {error:?}"
    );
    assert_eq!(
        outstanding,
        Some(1),
        "only one command may await source acknowledgement"
    );
    poll_until(&mut fixture.session, |session| {
        session.last_input_sequence >= 6
    });
    poll_until(&mut fixture.session, |session| {
        session.input.as_ref().unwrap().outstanding() == 0
    });
    assert_eq!(
        fixture.trace.lock().unwrap().sent,
        vec![
            Event::Move {
                point: [0.125, 0.5]
            },
            Event::Move {
                point: [0.625, 0.5]
            },
            Event::Key {
                code: 65,
                down: true
            },
            Event::Wheel {
                point: [0.5, 0.5],
                delta: 120,
                horizontal: false
            },
            Event::Key {
                code: 65,
                down: false
            },
            Event::Move { point: [0.75, 0.5] },
        ]
    );
    assert!(fixture.session.input_enabled());
    assert!(fixture.session.take_input_error().is_none());
    assert!(fixture.session.take_input_notice().is_none());
}

#[test]
fn resume_waits_for_stopping_worker_before_sending_new_input() {
    let _apartment = Apartment::new().unwrap();
    let mut fixture = Fixture::with_release_gate(true);
    fixture.message(WM_KEYDOWN, 65, 0);
    poll_until(&mut fixture.session, |session| {
        session.input.as_ref().unwrap().outstanding() == 0
    });
    // 对应缩放/最小化取消后紧接新鼠标消息，旧 worker 仍在释放已按住的键。
    fixture
        .session
        .window
        .as_ref()
        .unwrap()
        .suspend_source_input();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture
        .session
        .window
        .as_ref()
        .unwrap()
        .request_default_input();
    fixture.motion(64);
    assert!(!fixture.session.input_enabled());
    assert!(fixture.session.take_input_error().is_none());
    assert!(fixture.session.take_input_notice().is_none());
    assert!(fixture
        .session
        .window
        .as_ref()
        .unwrap()
        .input_resume_pending());
    fixture.release.send(()).unwrap();
}

#[test]
fn focus_loss_cancels_pending_motion_and_releases_held_key() {
    let _apartment = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    fixture.message(WM_KEYDOWN, 65, 0);
    poll_until(&mut fixture.session, |session| {
        session.input.as_ref().unwrap().outstanding() == 0
    });
    fixture.block_motion();
    for x in 20..100 {
        fixture.motion(x);
    }
    fixture.message(WM_KEYDOWN, 66, 0);
    fixture.message(WM_KILLFOCUS, 0, 0);
    fixture.release.send(()).unwrap();
    poll_until(&mut fixture.session, |session| session.input.is_none());
    let trace = fixture.trace.lock().unwrap();
    assert_eq!(
        trace.sent,
        vec![
            Event::Key {
                code: 65,
                down: true
            },
            Event::Move {
                point: [0.125, 0.5]
            }
        ]
    );
    assert_eq!(
        trace.released,
        vec![Event::Key {
            code: 65,
            down: false
        }]
    );
    assert!(!fixture.session.input_enabled());
    assert!(fixture.session.take_input_error().is_none());
    assert_eq!(fixture.session.input_cleanup().unwrap().released, 1);
}

#[test]
fn high_resolution_wheel_burst_preserves_control_and_total_delta() {
    let _apartment = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    fixture.block_motion();
    let mut point = POINT { x: 64, y: 64 };
    unsafe {
        assert!(ClientToScreen(fixture.hwnd(), &mut point).as_bool());
    }
    let position =
        (u32::from(point.x as i16 as u16) | (u32::from(point.y as i16 as u16) << 16)) as isize;
    // 慢源等待期间，触控板/高分辨率滚轮的一次手势可产生远多于 16 个小 delta。
    for _ in 0..100 {
        fixture.message(WM_MOUSEWHEEL, 30 << 16, position);
    }
    fixture.message(WM_KEYDOWN, 65, 0);
    fixture.message(WM_KEYUP, 65, 0);
    fixture.release.send(()).unwrap();
    poll_until(&mut fixture.session, |session| {
        session.input.is_none()
            || (session.last_input_sequence >= 6
                && session
                    .input
                    .as_ref()
                    .is_some_and(|worker| worker.outstanding() == 0))
    });
    assert!(
        fixture.session.input_enabled(),
        "wheel backlog revoked control"
    );
    let trace = fixture.trace.lock().unwrap();
    let mut total = 0;
    for event in &trace.sent[1..trace.sent.len() - 2] {
        let Event::Wheel {
            point,
            delta,
            horizontal,
        } = event
        else {
            panic!("unexpected wheel burst event: {event:?}");
        };
        assert_eq!(*point, [0.5, 0.5]);
        assert!(!horizontal);
        assert!((1..=1200).contains(delta));
        total += delta;
    }
    assert_eq!(total, 3000);
    assert_eq!(
        &trace.sent[trace.sent.len() - 2..],
        &[
            Event::Key {
                code: 65,
                down: true
            },
            Event::Key {
                code: 65,
                down: false
            },
        ]
    );
}
