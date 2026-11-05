//! Live 时间点检查点：真实 WARP 帧、磁盘与隐藏恢复，不捕获用户桌面或重连权限。
use super::*;
use crate::desktop::workspace_profile::Profile;
use std::path::PathBuf;
#[path = "application_live_checkpoint_autosave_tests.rs"]
mod autosave;
#[path = "application_live_checkpoint_crash_tests.rs"]
mod crash;
#[path = "application_live_checkpoint_exit_tests.rs"]
mod exit;
#[path = "application_live_checkpoint_lifecycle_tests.rs"]
mod lifecycle;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-live-checkpoint-tests",
        );
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path) // 保留隔离文件作为失败/成功证据，不递归清理。
    }
    fn path(&self) -> PathBuf {
        self.0.join("workspace.json")
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.path()).unwrap()
    }
}
unsafe fn drain() {
    let mut message = MSG::default();
    for _ in 0..1024 {
        if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            return;
        }
        if message.message != WM_QUIT {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    panic!("Live checkpoint message drain exceeded existing smoke bound");
}
unsafe fn app(environment: &Environment) -> Application {
    drain();
    let image = crate::desktop::image::Image::from_encoded(
        crate::desktop::application::smoke::image_fixture::PNG,
    )
    .unwrap();
    Application::new(
        environment.instance,
        DocumentWindow::create(
            environment.instance,
            State::new(Some(image), false).unwrap(),
            false,
        )
        .unwrap(),
    )
}
unsafe fn install(app: &mut Application, directory: &Directory, startup: bool) {
    app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), startup)
        .unwrap();
}
unsafe fn tick(app: &mut Application) {
    app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
}
unsafe fn ready(app: &Application) {
    let end = Instant::now() + Duration::from_secs(5);
    while !app.save_job.as_ref().unwrap().1.is_ready() {
        assert!(Instant::now() < end, "Live save deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
unsafe fn settle_auto(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        tick(app);
        if app.save_job.is_none() {
            return;
        }
        assert!(Instant::now() < end, "Live autosave deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
unsafe fn save_manual(app: &mut Application, path: PathBuf) {
    app.request_save_new(path).unwrap();
    super::save::prepared(app)
        .unwrap()
        .commit_ready(app, &AtomicBool::new(false))
        .unwrap();
}
fn revision(bytes: &[u8]) -> u64 {
    crate::session_document::decode(bytes).unwrap().revision
}
fn pixels(bytes: &[u8], id: &crate::workspace_id::UnitId, color: [u8; 4]) {
    let candidate =
        crate::desktop::session_candidate::prepare(bytes.to_vec(), None, &AtomicBool::new(false))
            .unwrap();
    let image = candidate
        .images
        .iter()
        .find(|image| &image.unit == id)
        .unwrap()
        .source
        .as_ref()
        .unwrap();
    assert_eq!([image.width, image.height], [2, 2]);
    assert_eq!(&*image.pixels, &color.repeat(4));
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let sticker = value["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["id"] == id.as_str())
        .unwrap();
    for field in [
        "target",
        "hwnd",
        "liveBinding",
        "captureMeta",
        "execution",
        "parameters",
    ] {
        assert!(
            sticker.get(field).is_none_or(serde_json::Value::is_null),
            "{field}"
        );
    }
}
#[test]
fn first_frame_is_saved_once_stream_changes_do_not_write_and_static_recovery_keeps_graph() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (hwnd, id, source) = super::refresh::stream(&mut app);
        let group = crate::workspace_id::UnitId::parse("checkpoint-live-group").unwrap();
        app.workspace
            .upsert_group(crate::workspace::Group {
                id: group.clone(),
                name: "时间点".into(),
                hidden: false,
                locked: false,
            })
            .unwrap();
        app.workspace
            .assign_group(std::slice::from_ref(&id), Some(&group))
            .unwrap();
        let link = crate::workspace::Link {
            id: crate::workspace_id::UnitId::parse("checkpoint-live-link").unwrap(),
            from_unit: id.clone(),
            from_port: "image".into(),
            to_unit: app.documents[0].state.unit_id.clone(),
            to_port: "image".into(),
        };
        app.workspace.connect(link.clone()).unwrap();
        install(&mut app, &directory, false);
        settle_auto(&mut app);
        assert!(!directory.path().exists() && !app.checkpoint_pending());
        source.publish([0, 0, 255, 255]);
        settle_auto(&mut app);
        let first = directory.bytes();
        assert_eq!(revision(&first), 1);
        pixels(&first, &id, [0, 0, 255, 255]);
        source.publish([0, 255, 0, 255]);
        for _ in 0..4 {
            settle_auto(&mut app);
        }
        assert_eq!(directory.bytes(), first);
        SetWindowPos(
            hwnd,
            None,
            -240,
            160,
            120,
            100,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        settle_auto(&mut app);
        let second = directory.bytes();
        assert_eq!(revision(&second), 2);
        pixels(&second, &id, [0, 255, 0, 255]);
        assert!(app.restored_session.as_ref().unwrap().saved_path.is_none());
        assert_eq!(app.save_status(), save_state::Status::Unsaved);
        assert!(app.live_sessions[0].is_alive());
        drop(source);
        app.shutdown_live().unwrap();
        drop(app);
        let mut restored = self::app(&environment);
        install(&mut restored, &directory, true);
        assert!(restored.live_sessions.is_empty());
        assert_eq!(restored.workspace.group_of(&id).unwrap().id, group);
        assert_eq!(restored.workspace.links(), &[link]);
        assert!(restored
            .documents
            .iter()
            .all(|doc| !doc.state.global_capture.get()));
        assert!(restored
            .documents
            .iter()
            .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
        assert_eq!(
            restored
                .documents
                .iter()
                .filter(|doc| doc.state.unit_id == id)
                .count(),
            1
        );
    }
}

#[test]
fn checkpoint_does_not_change_manual_target_or_dirty_and_new_static_edit_takes_current_frame() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (hwnd, id, source) = super::refresh::stream(&mut app);
        source.publish([0, 0, 255, 255]);
        let manual = directory.0.join("manual.json");
        save_manual(&mut app, manual.clone());
        let before = std::fs::read(&manual).unwrap();
        let original = app.restored_session.as_ref().unwrap().original.clone();
        let target = app.restored_session.as_ref().unwrap().saved_path.clone();
        install(&mut app, &directory, false);
        settle_auto(&mut app);
        source.publish([255, 0, 0, 255]);
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
        crate::desktop::composition::live::window::local::tests::wheel(
            hwnd,
            &[windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU],
            -120,
            (0, 0),
        );
        settle_auto(&mut app);
        assert_eq!(revision(&directory.bytes()), 2);
        pixels(&directory.bytes(), &id, [255, 0, 0, 255]);
        assert_eq!(std::fs::read(&manual).unwrap(), before);
        let record = app.restored_session.as_ref().unwrap();
        assert_eq!(record.saved_path, target);
        assert!(record.original.same_storage(&original));
        assert_eq!(app.save_status(), save_state::Status::Changed);
        drop(source);
        app.shutdown_live().unwrap();
    }
}
