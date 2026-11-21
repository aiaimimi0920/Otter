//! 单槽 Live GPU 读回任务；完整帧和取消 owner 随线程持有，退出必须 join。
use crate::desktop::{
    loom_execution_signal::{CompletionSignal, Notify},
    open_image::PreparedDocument,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<PreparedDocument, String>>>,
}
impl Job {
    pub fn start(
        frame: Arc<crate::live_frames::Frame<crate::live_texture::Texture>>,
        editor: Option<crate::editor::Editor>,
        notify: Notify,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-live-freeze".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let image = crate::live_texture::readback::read(frame, &signal)
                    .map_err(|error| format!("Live readback failed: {error:?}"))?;
                let image = crate::image_decode::Image {
                    width: image.width,
                    height: image.height,
                    pixels: image.pixels,
                };
                let (editor, image) = if let Some(editor) = editor {
                    let (editor, image) =
                        crate::desktop::live_pin_snapshot::refresh(editor, image, || {
                            signal.load(Ordering::Acquire)
                        })?;
                    (Some(editor), image)
                } else {
                    (None, image)
                };
                let mut pin =
                    PreparedDocument::from_image(image, true).map_err(|error| error.to_string())?;
                pin.editable = editor.map(|editor| crate::desktop::clipboard_editable::Prepared {
                    editor,
                    opacity: 1.0,
                    frame_style: Default::default(),
                });
                if signal.load(Ordering::Acquire) {
                    return Err("Live freeze cancelled".into());
                }
                Ok(pin)
            })?;
        Ok(Self {
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn poll(&mut self) -> Option<Result<PreparedDocument, String>> {
        if !self.ready.load(Ordering::Acquire) && !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "Live freeze worker panicked".into())
                .and_then(|result| result),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
