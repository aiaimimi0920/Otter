//! 有界后台槽、退出排空、迟到载入与坏文件保护，不操作前台或真实profile。
use super::*;
fn root() -> PathBuf {
    let guid = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
    let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
        .join(format!("otter-history-worker-{guid:?}"));
    std::fs::create_dir(&root).unwrap();
    root
}
#[test]
fn shutdown_drains_capture_then_latest_color_save_and_reopens_both_lists() {
    let root = root();
    let view: Shared = Default::default();
    let mut owner = Owner::new(view.clone(), std::sync::Arc::new(|| {}));
    owner.install(Some(root.clone()));
    view.borrow_mut()
        .color(crate::stroke_style::Color(10, 20, 30));
    view.borrow_mut().capture(
        "capture".into(),
        crate::image_decode::Image {
            width: 2,
            height: 1,
            pixels: vec![0, 0, 255, 255, 0, 255, 0, 255].into(),
        },
    );
    owner.shutdown();
    assert!(owner.job.is_none());
    assert!(view.borrow().status.is_none());
    drop(owner);
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_, data) = Store::open(&root).unwrap();
    assert_eq!(data.colors[0].hex, "#0a141e");
    assert_eq!(data.screenshots[0].id, "capture");
}
#[test]
fn bad_load_keeps_session_history_usable_but_never_overwrites_source() {
    let root = root();
    std::fs::write(root.join("history.json"), b"broken").unwrap();
    let view: Shared = Default::default();
    let mut owner = Owner::new(view.clone(), std::sync::Arc::new(|| {}));
    owner.install(Some(root.clone()));
    view.borrow_mut().color(crate::stroke_style::Color(1, 2, 3));
    owner.shutdown();
    assert!(owner.failed);
    assert_eq!(view.borrow().data.colors.len(), 1);
    assert_eq!(std::fs::read(root.join("history.json")).unwrap(), b"broken");
}
#[test]
fn capture_queue_is_bounded_and_ordinary_images_do_not_enter_history() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let state = crate::desktop::State::new(
        Some(crate::image_decode::Image {
            width: 1,
            height: 1,
            pixels: vec![0; 4].into(),
        }),
        false,
    )
    .unwrap();
    crate::desktop::capture_history_ui::record(&state);
    assert!(state.capture_history.borrow().captures.is_empty());
    for index in 0..8 {
        state.capture_history.borrow_mut().capture(
            index.to_string(),
            state.canvas.borrow().image.clone().unwrap(),
        );
    }
    assert_eq!(state.capture_history.borrow().captures.len(), 4);
    assert!(state.capture_history.borrow().status.is_some());
}
