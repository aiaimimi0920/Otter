//! 隐藏 HWND、实际 worker/PNG/锁；仅替换最终 OLE 调用，不注入真实键鼠或系统剪贴板。
use super::*;
use std::{
    fs,
    time::{Duration, Instant},
};
use windows::core::w;
#[path = "drag_cancel_tests.rs"]
mod cancel_tests;
struct Fixture {
    root: PathBuf,
    hwnd: HWND,
}
impl Fixture {
    unsafe fn new() -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-drag-input-{:?}",
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        ));
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("drag test"),
            WS_OVERLAPPED,
            0,
            0,
            10,
            10,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        Self { root, hwnd }
    }
    fn files(&self) -> Vec<PathBuf> {
        let cache = self.root.join("clipboard_cache");
        if !cache.exists() {
            return vec![];
        }
        fs::read_dir(cache)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
        let cache = self.root.join("clipboard_cache");
        if cache.exists() {
            for entry in fs::read_dir(&cache).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
            fs::remove_dir(cache).unwrap();
        }
        if self.root.exists() {
            fs::remove_dir(&self.root).unwrap();
        }
    }
}
fn state() -> State {
    State::new(
        Some(crate::desktop::image::Image {
            width: 2,
            height: 2,
            pixels: vec![255; 16].into(),
        }),
        true,
    )
    .unwrap()
}
fn wait(state: &State) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !state
        .drag
        .pending
        .borrow()
        .as_ref()
        .unwrap()
        .job
        .is_finished()
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn threshold_handles_negative_coordinates_and_does_not_treat_click_as_drag() {
    assert!(!crossed((-10, -20), (-7, -17), (4, 4)));
    assert!(crossed((-10, -20), (-6, -20), (4, 4)));
    assert!(crossed((-10, -20), (-10, -24), (4, 4)));
    assert!(!crossed((0, 0), (0, 0), (0, 0)));
    assert_eq!(
        point(LPARAM(((0xfffeu32 << 16) | 0xffff) as isize)),
        (-1, -2)
    );
}

#[test]
fn drag_gesture_does_not_steal_shift_from_selection_or_drawing_tools() {
    let state = state();
    assert!(can_begin(&state));
    for tool in [
        crate::editor::Tool::Select,
        crate::editor::Tool::Brush,
        crate::editor::Tool::Rectangle,
        crate::editor::Tool::Text,
    ] {
        state.canvas.borrow_mut().select_tool(tool);
        assert!(!can_begin(&state));
    }
    state
        .canvas
        .borrow_mut()
        .select_tool(crate::editor::Tool::Pan);
    assert!(can_begin(&state));
}

#[test]
fn frozen_drag_pattern_publishes_once_with_no_ui_borrows_and_protects_source_during_ole() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        state.naming.borrow_mut().drag_export_pattern = "拖出_{kind}_{width}x{height}".into();
        state.naming.borrow_mut().sticker_save_pattern = "wrong-save".into();
        state.naming.borrow_mut().clipboard_file_pattern = "wrong-copy".into();
        start(fixture.hwnd, &state, Ok(fixture.root.clone())).unwrap();
        state.naming.borrow_mut().drag_export_pattern = "changed".into();
        wait(&state);
        let mut count = 0;
        finish::poll_using(fixture.hwnd, &state, true, |path, signal| {
            count += 1;
            assert_eq!(path.file_name().unwrap(), "拖出_image_2x2.png");
            assert!(state.file_dialog.get());
            assert!(state.drag.pending.borrow_mut().is_none());
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(!signal.load(Ordering::Acquire));
            assert!(fs::remove_file(&path).is_err());
            assert!(
                dispatch(fixture.hwnd, &state, WM_COMMAND, WPARAM(0x1040), LPARAM(0)).is_some()
            );
            dispatch(fixture.hwnd, &state, WM_CANCELMODE, WPARAM(0), LPARAM(0));
            assert!(signal.load(Ordering::Acquire));
            Ok(())
        });
        assert_eq!(count, 1);
        assert!(state.drag.active.borrow().is_none());
        assert!(!state.file_dialog.get());
        assert_eq!(fixture.files().len(), 1);
        finish::poll_using(fixture.hwnd, &state, true, |_, _| panic!("publish twice"));
    }
}

#[test]
fn released_button_stale_document_modal_and_closed_window_never_publish() {
    unsafe {
        for mode in 0..4 {
            let fixture = Fixture::new();
            let state = state();
            start(fixture.hwnd, &state, Ok(fixture.root.clone())).unwrap();
            wait(&state);
            match mode {
                1 => state.canvas.borrow_mut().normal_opacity = 0.5,
                2 => state.file_dialog.set(true),
                3 => {
                    DestroyWindow(fixture.hwnd).unwrap();
                }
                _ => (),
            }
            finish::poll_using(fixture.hwnd, &state, mode != 0, |_, _| {
                panic!("stale drag published")
            });
            assert!(state.drag.pending.borrow().is_none());
            assert!(fixture.files().is_empty());
        }
    }
}

#[test]
fn mouse_release_and_close_reclaim_pending_work_without_calling_ole() {
    unsafe {
        for message in [WM_LBUTTONUP, WM_CLOSE, WM_CANCELMODE, WM_CAPTURECHANGED] {
            let fixture = Fixture::new();
            let state = state();
            start(fixture.hwnd, &state, Ok(fixture.root.clone())).unwrap();
            wait(&state);
            dispatch(fixture.hwnd, &state, message, WPARAM(0), LPARAM(0));
            assert!(state.drag.pending.borrow().is_none());
            assert!(fixture.files().is_empty());
        }
    }
}

#[test]
fn transport_failure_releases_modal_and_file_guard_but_retains_published_cache() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        start(fixture.hwnd, &state, Ok(fixture.root.clone())).unwrap();
        wait(&state);
        finish::poll_using(fixture.hwnd, &state, true, |_, _| {
            Err("injected transport failure".into())
        });
        assert!(!state.file_dialog.get());
        assert!(state.drag.active.borrow().is_none());
        let files = fixture.files();
        assert_eq!(files.len(), 1);
        fs::remove_file(&files[0]).unwrap(); // 原生文件保护句柄已释放；仅删除本测试自己的 linshi 文件。
    }
}
