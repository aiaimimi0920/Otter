//! 跨线程纯移动只能由生产窗口自己的队列通知推进保存，不能借测试 tick 或 worker 句柄唤醒。
use super::*;
use std::cell::Cell;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

#[test]
fn background_checkpoint_keeps_workspace_borders_visible() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let hwnd = app.documents[0].window.0;
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            app.sync_workspace_chrome(true);
            let drawing = app.chrome.handles()[0];
            assert!(IsWindowVisible(drawing).as_bool());
            for opacity in [0.7, 0.5, 0.9] {
                app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
                app.pending_workspace_save_with(|_, _, message, warning| {
                    assert!(!warning, "{message}")
                });
                assert!(app.checkpoint_pending());
                assert!(app.save_job.is_some());
                app.sync_workspace_chrome(true);
                assert!(
                    IsWindowVisible(drawing).as_bool(),
                    "background checkpoint hid every border"
                );
                settle(&mut app);
            }
            app.sync_workspace_chrome(false);
            assert!(
                !IsWindowVisible(drawing).as_bool(),
                "explicit hide must still work"
            );
        });
    }
}

#[test]
fn sustained_ten_second_wheel_with_checkpoint_has_no_frame_backlog() {
    unsafe {
        run(|instance| {
            use crate::desktop::pin_window::{animation, preview};
            use windows::Win32::System::SystemServices::MK_CONTROL;
            use windows::Win32::UI::Input::KeyboardAndMouse::*;
            struct Keys([u8; 256]);
            impl Drop for Keys {
                fn drop(&mut self) {
                    unsafe { SetKeyboardState(&self.0).unwrap() };
                }
            }
            let mut original = [0; 256];
            GetKeyboardState(&mut original).unwrap();
            let _keys = Keys(original);
            let mut held = [0; 256];
            held[VK_CONTROL.0 as usize] = 0x80;
            SetKeyboardState(&held).unwrap();
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let hwnd = app.documents[0].window.0;
            app.documents[0].state.canvas.borrow_mut().pinned = true;
            SetWindowPos(
                hwnd,
                None,
                100,
                100,
                960,
                540,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            app.sync_workspace_chrome(true);
            settle(&mut app);
            drain_messages();
            let drawing = app.chrome.handles()[0];
            let address = hwnd.0 as usize;
            let (tx, rx) = std::sync::mpsc::channel();
            let mut stopped = None;
            let mut tail = Duration::ZERO;
            let mut hidden = false;
            let mut checkpoint_during_zoom = false;
            std::thread::scope(|scope| {
                scope.spawn(move || {
                    let started = Instant::now();
                    for index in 0..1000 {
                        let delta = if index % 10 < 5 { 30i16 } else { -30 };
                        PostMessageW(
                            Some(HWND(address as *mut _)),
                            WM_MOUSEWHEEL,
                            WPARAM(((delta as u16 as usize) << 16) | MK_CONTROL.0 as usize),
                            LPARAM(580 | (370 << 16)),
                        )
                        .unwrap();
                        std::thread::sleep(
                            (started + Duration::from_millis((index + 1) * 10))
                                .saturating_duration_since(Instant::now()),
                        );
                    }
                    tx.send(Instant::now()).unwrap();
                });
                let deadline = Instant::now() + Duration::from_secs(20);
                loop {
                    assert!(
                        Instant::now() < deadline,
                        "continuous input settlement deadline"
                    );
                    let mut message = MSG::default();
                    if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                        assert_ne!(message.message, WM_QUIT);
                        DispatchMessageW(&message);
                        app.pending_windows(true).unwrap();
                        app.pending_workspace_save_with(|_, _, message, warning| {
                            assert!(!warning, "{message}")
                        });
                    } else {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    hidden |= !IsWindowVisible(drawing).as_bool();
                    checkpoint_during_zoom |=
                        preview::active(&app.documents[0].state) && app.checkpoint_pending();
                    stopped = stopped.or_else(|| rx.try_recv().ok());
                    if let Some(at) = stopped {
                        if !PeekMessageW(
                            &mut message,
                            Some(hwnd),
                            WM_MOUSEWHEEL,
                            WM_MOUSEWHEEL,
                            PM_NOREMOVE,
                        )
                        .as_bool()
                        {
                            tail = at.elapsed();
                            break;
                        }
                    }
                }
            });
            let mut before = RECT::default();
            GetWindowRect(hwnd, &mut before).unwrap();
            animation::settle(hwnd, &app.documents[0].state);
            let mut after = RECT::default();
            GetWindowRect(hwnd, &mut after).unwrap();
            assert_eq!(before, after, "idle replayed an old zoom frame");
            settle(&mut app);
            eprintln!("continuous-wheel duration_ms=10000 packets=1000 tail_ms={} hidden={} checkpoint_during_zoom={}", tail.as_millis(), hidden, checkpoint_during_zoom);
            assert!(!hidden, "automatic persistence hid shared borders");
            assert!(
                !checkpoint_during_zoom,
                "automatic snapshot ran on the zoom hot path"
            );
            assert!(
                tail < Duration::from_millis(150),
                "wheel backlog remained after producer stopped: {tail:?}"
            );
        });
    }
}

struct Observer {
    hwnd: HWND,
    counts: Box<Cell<[u32; 2]>>,
}

#[test]
fn system_move_defers_checkpoint_until_release_without_hiding_borders() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let hwnd = app.documents[0].window.0;
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            app.sync_workspace_chrome(true);
            let drawing = app.chrome.handles()[0];
            SendMessageW(hwnd, WM_ENTERSIZEMOVE, None, None);
            SetWindowPos(
                hwnd,
                None,
                280,
                240,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            app.pending_workspace_save_with(|_, _, message, warning| {
                assert!(!warning, "{message}")
            });
            assert!(
                app.save_job.is_none(),
                "system drag must not snapshot intermediate geometry"
            );
            SendMessageW(hwnd, WM_EXITSIZEMOVE, None, None);
            app.pending_workspace_save_with(|_, _, message, warning| {
                assert!(!warning, "{message}")
            });
            assert!(app.checkpoint_pending());
            app.sync_workspace_chrome(true);
            assert!(IsWindowVisible(drawing).as_bool());
            settle(&mut app);
            let saved: serde_json::Value =
                serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
            assert_eq!(saved["stickers"][0]["x"].as_f64(), Some(280.0));
            assert_eq!(saved["stickers"][0]["y"].as_f64(), Some(240.0));
        });
    }
}
impl Observer {
    unsafe fn new(hwnd: HWND) -> Self {
        let counts = Box::new(Cell::new([0, 0]));
        assert!(
            SetWindowSubclass(hwnd, Some(observe), 1, (&*counts as *const _) as usize).as_bool()
        );
        Self { hwnd, counts }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(observe), 1);
        }
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let counts = &*(data as *const Cell<[u32; 2]>);
    let mut value = counts.get();
    if message == WM_MOVE {
        value[0] = value[0].saturating_add(1);
    } else if message == WM_SIZE {
        value[1] = value[1].saturating_add(1);
    }
    counts.set(value);
    DefSubclassProc(hwnd, message, wp, lp)
}

unsafe fn move_from_other_thread(hwnd: HWND, x: i32, y: i32) {
    let address = hwnd.0 as usize;
    let worker = std::thread::spawn(move || {
        SetWindowPos(
            HWND(address as *mut _),
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .is_ok()
    });
    let end = Instant::now() + Duration::from_secs(10);
    let mut message = MSG::default();
    // Peek 会执行 sent messages，但此过滤不领取任何产品 queued message。
    while !worker.is_finished() && Instant::now() < end {
        let _ = PeekMessageW(&mut message, None, WM_APP + 999, WM_APP + 999, PM_NOREMOVE);
        std::thread::yield_now();
    }
    let completed = worker.is_finished();
    if !completed {
        // 只销毁本测试拥有的窗口，以释放可能仍在等待 sent message 的 worker。
        DestroyWindow(hwnd).unwrap();
    }
    assert!(worker.join().unwrap(), "owned window move failed");
    assert!(completed, "owned window move deadline");
}

unsafe fn dispatch_and_settle(app: &mut Application, message: &MSG) {
    assert_ne!(message.message, WM_QUIT);
    let _ = TranslateMessage(message);
    DispatchMessageW(message);
    app.pending_windows(false).unwrap();
    app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
}

#[test]
fn sent_only_window_move_queues_settlement_and_persists_without_test_tick() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app); // 仅建立初始基线，移动之后不再主动 tick。
            let before = std::fs::read(directory.path()).unwrap();
            let revision = crate::session_document::decode(&before).unwrap().revision;
            assert!(!drain_messages());
            let hwnd = app.documents[0].window.0;
            assert!(!IsWindowVisible(hwnd).as_bool());
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).unwrap();
            let (x, y) = (rect.left + 2, rect.top + 3);
            move_from_other_thread(hwnd, x, y);
            GetWindowRect(hwnd, &mut rect).unwrap();
            assert_eq!((rect.left, rect.top), (x, y));
            assert_eq!(std::fs::read(directory.path()).unwrap(), before);
            assert!(app.save_job.is_none());
            let mut message = MSG::default();
            assert!(
                PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool(),
                "sent-only geometry change did not queue application settlement"
            );
            dispatch_and_settle(&mut app, &message);
            let end = Instant::now() + Duration::from_secs(10);
            loop {
                let bytes = std::fs::read(directory.path()).unwrap();
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                if value["documentRevision"].as_u64() == Some(revision + 1) {
                    assert_eq!(value["stickers"][0]["x"].as_f64(), Some(x as f64));
                    assert_eq!(value["stickers"][0]["y"].as_f64(), Some(y as f64));
                    assert!(app.save_job.is_none());
                    break;
                }
                assert!(Instant::now() < end, "queued geometry checkpoint deadline");
                if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    dispatch_and_settle(&mut app, &message);
                } else {
                    std::thread::yield_now();
                }
            }
            drop(app);
        });
    }
}

#[test]
fn real_window_coalesces_moves_rearms_and_preserves_default_size_dispatch() {
    unsafe {
        run(|instance| {
            let app = new_app(instance);
            assert!(!drain_messages());
            let hwnd = app.documents[0].window.0;
            let observer = Observer::new(hwnd);
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).unwrap();
            for offset in 1..=100 {
                SetWindowPos(
                    hwnd,
                    None,
                    rect.left + offset,
                    rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
            }
            let mut message = MSG::default();
            assert!(PeekMessageW(&mut message, Some(hwnd), WM_APP, 0xBFFF, PM_REMOVE).as_bool());
            let id = message.message;
            let first = message;
            assert!(!PeekMessageW(&mut message, Some(hwnd), id, id, PM_REMOVE).as_bool());
            DispatchMessageW(&first);
            assert_eq!(observer.counts.get(), [100, 0]);
            // 隐藏窗口的 invalid region 不是派发凭证；直接观察真实 DefWindowProc 的 WM_SIZE。
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                rect.right - rect.left + 1,
                rect.bottom - rect.top + 1,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            assert_eq!(observer.counts.get(), [100, 1]);
            assert!(PeekMessageW(&mut message, Some(hwnd), id, id, PM_REMOVE).as_bool());
            DispatchMessageW(&message);
            assert!(!PeekMessageW(&mut message, Some(hwnd), id, id, PM_REMOVE).as_bool());
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .unwrap();
            assert!(!PeekMessageW(&mut message, Some(hwnd), id, id, PM_REMOVE).as_bool());
            assert_eq!(observer.counts.get(), [100, 1]);
            drop(observer); // 必须先撤 subclass，再销毁 HWND/释放计数地址。
            drop(app);
        });
    }
}
