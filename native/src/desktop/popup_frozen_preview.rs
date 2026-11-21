//! 仅缓存视口附近小图；共享不可变会话，最多一个可取消任务、每库至多三个批次。
use crate::{
    image_thumbnail::Thumbnail,
    session_library::{self, Kind},
    session_source::Source,
};
use std::{
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

struct Images {
    library: usize,
    range: Range<usize>,
    entries: Vec<(String, Option<Thumbnail>)>,
}
struct Job {
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Images>>,
}
#[derive(Default)]
pub(super) struct Previews {
    ready: [Option<Images>; 2],
    job: Option<Job>,
    requested: Option<(usize, Range<usize>)>,
    dirty: [bool; 2],
}
impl Previews {
    pub fn invalidate(&mut self) {
        self.job = None;
        self.requested = None;
        // 保留按 ID 绑定的小图供原帧使用，但新来源必须重新填满移动后的范围。
        for ready in self.ready.iter_mut().flatten() {
            ready.range = 0..0;
        }
    }
    pub fn image(&self, library: usize, id: &str) -> Option<Thumbnail> {
        self.ready[library]
            .as_ref()?
            .entries
            .iter()
            .find(|(key, _)| key == id)?
            .1
            .clone()
    }
    pub fn update(
        &mut self,
        source: &Source,
        library: usize,
        view: Range<usize>,
        count: usize,
    ) -> bool {
        // 快速滚回已缓存区域时，废弃离开该区域后的请求，不能让迟到结果挤走当前小图。
        if self.ready[library]
            .as_ref()
            .is_some_and(|ready| covers(&ready.range, &view))
            && self
                .requested
                .as_ref()
                .is_some_and(|(slot, range)| *slot != library || !covers(range, &view))
        {
            self.job = None;
            self.requested = None;
        }
        if self.job.as_ref().is_some_and(|job| {
            job.worker
                .as_ref()
                .is_some_and(|worker| worker.is_finished())
        }) {
            let mut job = self.job.take().unwrap();
            if let Some(ready) = job.worker.take().and_then(|worker| worker.join().ok()) {
                let slot = ready.library;
                self.ready[slot] = Some(ready);
                self.dirty[slot] = true;
            }
        }
        let changed = std::mem::take(&mut self.dirty[library]);
        if self.ready[library]
            .as_ref()
            .is_some_and(|ready| covers(&ready.range, &view))
            || self
                .requested
                .as_ref()
                .is_some_and(|(slot, range)| *slot == library && covers(range, &view))
        {
            return changed;
        }
        // 跨出缓存范围取消原工作，不积累队列；解码前后均观察取消。
        self.job = None;
        let start = (view.start / super::BATCH).saturating_sub(1) * super::BATCH;
        let range = start..(start + super::BATCH * 3).min(count);
        self.requested = Some((library, range.clone()));
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let source = source.clone();
        if let Ok(worker) = std::thread::Builder::new()
            .name("otter-menu-preview".into())
            .spawn(move || {
                let mut ready = Images {
                    library,
                    range: range.clone(),
                    entries: Vec::new(),
                };
                if signal.load(Ordering::Acquire) {
                    return ready;
                }
                if let Ok(data) = session_library::decode(
                    &source,
                    [Kind::RecycleBin, Kind::ReferenceLibrary][library],
                ) {
                    for entry in data.entries().iter().skip(range.start).take(range.len()) {
                        if signal.load(Ordering::Acquire) {
                            break;
                        }
                        // 仅现有内联资源，绝不因缩略图自动授权文件路径或联网。
                        let image = [
                            entry.window.preview_src.as_deref(),
                            entry.window.src.as_deref(),
                        ]
                        .into_iter()
                        .flatten()
                        .find_map(|uri| image(uri, &signal));
                        ready.entries.push((entry.entry_id.to_string(), image));
                    }
                }
                ready
            })
        {
            self.job = Some(Job {
                cancel,
                worker: Some(worker),
            });
        }
        changed
    }
}
fn covers(range: &Range<usize>, view: &Range<usize>) -> bool {
    range.start <= view.start && range.end >= view.end
}
fn image(uri: &str, cancel: &AtomicBool) -> Option<Thumbnail> {
    const MAX: usize = 16 * 1024 * 1024;
    if uri.len() > MAX.div_ceil(3) * 4 + 64 {
        return None;
    }
    let (header, encoded) = uri.split_once(',')?;
    let mime = header.strip_prefix("data:")?.strip_suffix(";base64")?;
    if !matches!(mime, "image/png" | "image/jpeg" | "image/gif" | "image/bmp")
        || encoded.len() > MAX.div_ceil(3) * 4
    {
        return None;
    }
    let bytes = crate::desktop::loom_output_inline::decode_bytes(mime, encoded, cancel).ok()?;
    let _com = crate::desktop::Apartment::new().ok()?;
    let image =
        crate::image_decode::Image::from_menu_thumbnail(&bytes, &|| cancel.load(Ordering::Acquire))
            .ok()?;
    Thumbnail::from_image(&image).ok()
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
#[path = "popup_frozen_preview_tests.rs"]
mod tests;
