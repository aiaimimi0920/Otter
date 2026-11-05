//! 隐藏真实 HWND 与合成 HDR 数据验证等待窗口不提前发布，取消清理跨线程暂存。
use super::*;
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(
            PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
                .join(format!("otter-wait-cached-{:?}", unsafe {
                    windows::Win32::System::Com::CoCreateGuid().unwrap()
                })),
        )
    }
    fn entries(&self) -> usize {
        fs::read_dir(self.0.join("clipboard_cache"))
            .unwrap()
            .count()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let cache = self.0.join("clipboard_cache");
        if cache.exists() {
            for file in fs::read_dir(&cache).unwrap() {
                fs::remove_file(file.unwrap().path()).unwrap();
            }
            fs::remove_dir(cache).unwrap();
        }
        if self.0.exists() {
            fs::remove_dir(&self.0).unwrap();
        }
    }
}
fn prepare(profile: &std::path::Path, cancel: &AtomicBool) -> Result<PreparedCapture, String> {
    let bytes: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let source = crate::capture_color::ScRgb::new(&bytes, 1, 1, 8).unwrap();
    let mut result = outcome();
    result.frame = Frame::Hdr {
        image: source.to_hdr(0.01, 1000.0, 1.0, cancel).unwrap(),
        preview: source.to_sdr(320.0, 1.0, cancel).unwrap(),
    };
    PreparedCapture::prepare(result, profile, "wait", Default::default(), cancel)
        .map_err(|error| error.to_string())
}
#[test]
fn hidden_cached_dialog_returns_unpublished_result_for_owner_acceptance() {
    unsafe {
        let directory = Directory::new();
        let profile = directory.0.clone();
        let input = Input::new(
            Box::new(move || CaptureJob::spawn(move |cancel| prepare(&profile, cancel))),
            false,
        );
        let window = hidden(&input);
        settle(window.0, &input);
        assert!(input.job.borrow().is_none());
        assert_eq!(directory.entries(), 2);
        let prepared = input.result.borrow_mut().take().unwrap().unwrap().unwrap();
        drop(window);
        assert_eq!(directory.entries(), 2);
        let captured = prepared.commit(&AtomicBool::new(false), || true).unwrap();
        assert_eq!(captured.preview.pixels[0], 188);
        assert!(captured.original.unwrap().is_file());
        assert_eq!(directory.entries(), 3);
    }
}
#[test]
fn hidden_cached_dialog_close_discards_pending_file_after_worker_returns() {
    unsafe {
        let directory = Directory::new();
        let profile = directory.0.clone();
        let (ready, entered) = mpsc::sync_channel(1);
        let (release, wait) = mpsc::sync_channel(1);
        let input = Input::new(
            Box::new(move || {
                CaptureJob::spawn(move |cancel| {
                    let result = prepare(&profile, cancel)?;
                    ready.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(5)).unwrap();
                    Ok(result)
                })
            }),
            false,
        );
        let window = hidden(&input);
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(directory.entries(), 2);
        SendMessageW(window.0, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        assert!(input.result.borrow().is_none());
        assert_eq!(directory.entries(), 2);
        release.send(()).unwrap();
        settle(window.0, &input);
        assert!(matches!(input.result.borrow_mut().take(), Some(Ok(None))));
        assert_eq!(directory.entries(), 1);
    }
}
