//! 可控真实 GPU 帧流验证菜单刷新、固定帧边界、通知策略及失败保留；不捕获用户桌面。
use super::*;
pub(super) struct Stream {
    send: std::sync::mpsc::Sender<Option<crate::live_texture::Texture>>,
    ready: std::sync::mpsc::Receiver<()>,
}
impl Stream {
    pub(super) unsafe fn publish(&self, color: [u8; 4]) {
        self.send.send(Some(texture_with_color(color))).unwrap();
        self.ready.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.send.send(None);
    }
}
pub(super) unsafe fn stream(app: &mut Application) -> (HWND, crate::workspace_id::UnitId, Stream) {
    use crate::desktop::composition::live::window::LiveWindow;
    let (send, receive) = std::sync::mpsc::channel();
    let stop = send.clone();
    let (ready, wait) = std::sync::mpsc::channel();
    let (job, window) = crate::live_job::LiveJob::spawn_bound(
        &Arc::new(AtomicUsize::new(0)),
        |consumer| LiveWindow::new(consumer, 64, 64).map_err(std::io::Error::other),
        move |source, _| {
            source
                .on_idle(Arc::new(move || {
                    let _ = stop.send(None);
                }))
                .unwrap();
            while let Some(frame) = receive.recv_timeout(Duration::from_secs(10)).unwrap() {
                source.publish(frame).unwrap();
                ready.send(()).unwrap();
            }
            Ok(())
        },
    )
    .unwrap();
    let hwnd = window.hwnd_for_test();
    let session = Session::new(job, window);
    let unit = session.unit_id().clone();
    app.admit_live(session, false).unwrap();
    (hwnd, unit, Stream { send, ready: wait })
}
unsafe fn refresh(hwnd: HWND) {
    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fb0)), Some(LPARAM(0)));
}
fn settle(app: &Application, color: [u8; 4]) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.sync_display_frames();
        if app.documents[0]
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .is_some_and(|frame| *frame.image.pixels == color.repeat(4))
        {
            break;
        }
        assert!(Instant::now() < deadline, "Live refresh timeout");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn clear_notice(app: &Application) {
    app.documents[0].state.pending_start.borrow_mut().take();
    app.documents[0].state.execution_cancel.borrow_mut().take();
}
#[test]
fn live_refresh_menu_commits_latest_frame_and_notifies_once_not_per_stream_frame() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (_listener, _) = super::execution::select(&app);
        let (hwnd, unit, source) = stream(&mut app);
        source.publish([0, 0, 255, 255]);
        super::execution::connect(&mut app, &unit);
        settle(&app, [0, 0, 255, 255]);
        assert_eq!(
            app.documents[0]
                .state
                .pending_start
                .borrow()
                .as_ref()
                .unwrap()
                .trigger,
            crate::graph_execution_config::Trigger::Upstream
        );
        clear_notice(&app);
        source.publish([0, 255, 0, 255]);
        for _ in 0..8 {
            app.sync_display_frames();
        }
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        assert_eq!(
            &*app.documents[0]
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .image
                .pixels,
            &[0, 0, 255, 255].repeat(4)
        );
        refresh(hwnd);
        // 尚未调度的重复菜单请求合并；应用处理时只捕获最新蓝帧。
        source.publish([255, 0, 0, 255]);
        refresh(hwnd);
        app.sync_display_frames();
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        assert_eq!(
            &*app.documents[0]
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .image
                .pixels,
            &[0, 0, 255, 255].repeat(4)
        );
        // worker 已拿到固定蓝帧，之后流变绿也不能改变本次刷新结果。
        source.publish([0, 255, 0, 255]);
        settle(&app, [255, 0, 0, 255]);
        assert!(app.documents[0].state.pending_start.borrow().is_some());
        clear_notice(&app);
        for _ in 0..8 {
            app.sync_display_frames();
        }
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_refresh_readback_failure_preserves_old_pixels_and_sends_no_notice() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (_listener, _) = super::execution::select(&app);
        let (_, unit) = live(&mut app, true);
        super::execution::connect(&mut app, &unit);
        settle(&app, [0, 0, 255, 255]);
        clear_notice(&app);
        let identity = app.documents[0]
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .clone();
        let mut invalid = texture();
        invalid.width = 3; // 真实纹理仍 2×2，读回必须拒绝描述符不一致。
        app.display_composition
            .borrow_mut()
            .refresh_live(
                unit,
                app.live_sessions[0].input_identity(),
                Arc::new(crate::live_frames::Frame {
                    sequence: 99,
                    resource: invalid,
                }),
            )
            .unwrap();
        app.sync_display_frames();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.display_composition.borrow().is_busy() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
            app.sync_display_frames();
        }
        assert!(Arc::ptr_eq(
            &identity,
            &app.documents[0]
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .identity
        ));
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_refresh_respects_downstream_policy_and_source_lock_and_replacement() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (_listener, _) = super::execution::select(&app);
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .execution
            .listen_upstream = false;
        let (hwnd, unit, source) = stream(&mut app);
        source.publish([0, 0, 255, 255]);
        super::execution::connect(&mut app, &unit);
        settle(&app, [0, 0, 255, 255]);
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        let group = crate::workspace_id::UnitId::parse("locked-live-refresh").unwrap();
        app.workspace
            .upsert_group(crate::workspace::Group {
                id: group.clone(),
                name: "锁定".into(),
                hidden: false,
                locked: true,
            })
            .unwrap();
        app.workspace
            .assign_group(std::slice::from_ref(&unit), Some(&group))
            .unwrap();
        app.sync_groups();
        source.publish([0, 255, 0, 255]);
        refresh(hwnd);
        for _ in 0..8 {
            app.sync_display_frames();
        }
        assert!(!app.display_composition.borrow().is_busy());
        assert_eq!(
            &*app.documents[0]
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .image
                .pixels,
            &[0, 0, 255, 255].repeat(4)
        );
        app.workspace.remove_group(&group);
        app.sync_groups();
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .execution
            .listen_upstream = true;
        refresh(hwnd);
        app.sync_display_frames();
        let links = app.workspace.links().to_vec();
        app.workspace =
            crate::workspace::Workspace::with_initial(app.documents[0].state.unit_id.clone());
        app.workspace.add_units([unit]).unwrap();
        app.workspace.connect(links[0].clone()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.sync_display_frames();
            if !app.display_composition.borrow().is_busy() {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.documents[0].state.resolved_display.borrow().is_none());
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        app.shutdown_live().unwrap();
    }
}
