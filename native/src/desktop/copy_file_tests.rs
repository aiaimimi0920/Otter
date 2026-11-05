//! 真实隐藏 HWND 与后台缓存链；替换最终发布回调，不访问系统剪贴板或用户配置。
use super::*;
#[path = "copy_order_tests.rs"]
mod order_tests;
use std::{fs, path::PathBuf};
struct Fixture {
    root: PathBuf,
    hwnd: HWND,
}
impl Fixture {
    unsafe fn new() -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-smart-copy-{:?}",
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        ));
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Otter copy test"),
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
    fn plan(&self) -> file_plan::Plan {
        file_plan::Plan {
            root: self.root.clone(),
            stem: "复制🙂".into(),
            policy: Default::default(),
        }
    }
    fn files(&self) -> Vec<PathBuf> {
        let directory = self.root.join("clipboard_cache");
        if !directory.exists() {
            return vec![];
        }
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|value| value == "png"))
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
        let directory = self.root.join("clipboard_cache");
        if directory.exists() {
            for entry in fs::read_dir(&directory).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
            fs::remove_dir(directory).unwrap();
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
        false,
    )
    .unwrap()
}
unsafe fn finish(fixture: &Fixture, state: &State) -> usize {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut published = 0;
    while state.copy_job.borrow().is_some() {
        assert!(std::time::Instant::now() < deadline);
        poll_files_using(fixture.hwnd, state, |owner, payload, paths| {
            assert_eq!(owner, fixture.hwnd);
            assert_eq!(paths.len(), 1);
            assert_eq!(fs::read(&paths[0]).unwrap(), &*payload.png);
            assert!(payload.dibv5.len() >= 124);
            assert!(crate::desktop::paste_input::file::encode(paths).is_ok());
            published += 1;
            Ok(())
        });
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    published
}
#[test]
fn smart_copy_publishes_completed_file_once_and_retains_it_after_job_and_window_state() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        start_with_plan(fixture.hwnd, &state, Some(fixture.plan()));
        start_with_plan(fixture.hwnd, &state, Some(fixture.plan()));
        assert_eq!(finish(&fixture, &state), 1);
        let first = fixture.files();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].file_name().unwrap(), "复制🙂.png");
        start_with_plan(fixture.hwnd, &state, Some(fixture.plan()));
        assert_eq!(finish(&fixture, &state), 1);
        drop(state);
        assert_eq!(fixture.files().len(), 2);
        assert!(fixture.root.join("clipboard_cache/复制🙂_2.png").exists());
    }
}
#[test]
fn cancelled_or_replaced_document_never_publishes_backing_file() {
    unsafe {
        for cancelled in [true, false] {
            let fixture = Fixture::new();
            let state = state();
            start_with_plan(fixture.hwnd, &state, Some(fixture.plan()));
            if cancelled {
                cancel(&state);
            } else {
                let next = state.canvas.borrow().editor.export_snapshot();
                state.canvas.borrow_mut().editor = next;
            }
            assert_eq!(finish(&fixture, &state), 0);
            assert!(fixture.files().is_empty());
        }
    }
}
#[test]
fn clipboard_pattern_is_distinct_from_save_pattern_and_uses_actual_dimensions() {
    let state = state();
    {
        let mut settings = state.naming.borrow_mut();
        settings.sticker_save_pattern = "保存_{width}".into();
        settings.clipboard_file_pattern = "复制_{width}x{height}_{kind}".into();
    }
    assert_eq!(
        crate::desktop::export_naming::suggested(&state).as_deref(),
        Some("保存_2.png")
    );
    assert_eq!(
        crate::desktop::export_naming::suggested_clipboard(&state).as_deref(),
        Some("复制_2x2_image.png")
    );
}
