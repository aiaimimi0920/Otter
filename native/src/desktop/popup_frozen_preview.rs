//! 仅缓存视口附近小图；共享不可变会话，最多一个可取消任务、每库至多三个批次。
use super::worker::{self, Slot};
use crate::{
    desktop::loom_execution_signal::Notify,
    image_thumbnail::Thumbnail,
    session_library::{self, Kind},
    session_source::Source,
};
use std::{
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) struct Images {
    library: usize,
    range: Range<usize>,
    entries: Vec<(String, Option<Thumbnail>)>,
}
struct Job {
    library: usize,
    range: Range<usize>,
    worker: worker::Job<Images>,
}
#[derive(Default)]
pub(super) struct Previews {
    ready: [Option<Images>; 2],
    job: Option<Job>,
    slot: Slot<Images>,
    notify: Option<Notify>,
}
impl Previews {
    pub fn new(slot: Slot<Images>, notify: Option<Notify>) -> Self {
        Self {
            ready: Default::default(),
            job: None,
            slot,
            notify,
        }
    }

    pub fn invalidate(&mut self) {
        if let Some(job) = &self.job {
            job.worker.cancel.store(true, Ordering::Release);
        }
        // 保留按 ID 绑定的小图供原帧使用，但新来源必须重新填满移动后的范围。
        for ready in self.ready.iter_mut().flatten() {
            ready.range = 0..0;
        }
    }
    #[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
    pub fn image(&self, library: usize, id: &str) -> Option<Thumbnail> {
        self.ready[library]
            .as_ref()?
            .entries
            .iter()
            .find(|(key, _)| key == id)?
            .1
            .clone()
    }
    /// Offsets describe a just-published batch; after invalidation only IDs remain valid.
    pub fn batch(&self, library: usize) -> impl Iterator<Item = (usize, &str, &Thumbnail)> {
        self.ready[library].iter().flat_map(|ready| {
            ready
                .entries
                .iter()
                .enumerate()
                .filter_map(|(offset, (id, image))| {
                    Some((ready.range.start + offset, id.as_str(), image.as_ref()?))
                })
        })
    }
    pub fn update(
        &mut self,
        source: &Source,
        library: usize,
        view: Range<usize>,
        count: usize,
    ) -> bool {
        self.update_with(
            source,
            library,
            view,
            count,
            |slot, library, range, source, notify| {
                slot.start("otter-menu-preview", notify, move |cancel| {
                    decode(library, range, source, cancel)
                })
            },
        )
    }
    fn update_with(
        &mut self,
        source: &Source,
        library: usize,
        view: Range<usize>,
        count: usize,
        start_worker: impl FnOnce(
            &Slot<Images>,
            usize,
            Range<usize>,
            Source,
            Option<Notify>,
        ) -> std::io::Result<Option<worker::Job<Images>>>,
    ) -> bool {
        // 快速滚回已缓存区域时，废弃离开该区域后的请求，不能让迟到结果挤走当前小图。
        let cached = self.ready[library]
            .as_ref()
            .is_some_and(|ready| covers(&ready.range, &view));
        if let Some(job) = &self.job {
            if job.library != library || !covers(&job.range, &view) {
                job.worker.cancel.store(true, Ordering::Release);
            }
        }
        let mut changed = false;
        if let Some(result) = self.job.as_mut().and_then(|job| job.worker.poll()) {
            let job = self.job.take().unwrap();
            if let Ok(ready) = result {
                if !job.worker.cancel.load(Ordering::Acquire) {
                    let slot = ready.library;
                    self.ready[slot] = Some(ready);
                    changed = true;
                }
            }
        }
        // 取消只标记；旧线程真正结束前不等待，也不为新视口并行创建任务。
        if cached || changed || self.job.is_some() {
            return changed;
        }
        let start = (view.start / super::BATCH).saturating_sub(1) * super::BATCH;
        let range = start..(start + super::BATCH * 3).min(count);
        if let Ok(Some(worker)) = start_worker(
            &self.slot,
            library,
            range.clone(),
            source.clone(),
            self.notify.clone(),
        ) {
            self.job = Some(Job {
                library,
                range,
                worker,
            });
        }
        changed
    }
}
fn decode(library: usize, range: Range<usize>, source: Source, signal: &AtomicBool) -> Images {
    let mut ready = Images {
        library,
        range: range.clone(),
        entries: Vec::new(),
    };
    if signal.load(Ordering::Acquire) {
        return ready;
    }
    if let Ok(data) =
        session_library::decode(&source, [Kind::RecycleBin, Kind::ReferenceLibrary][library])
    {
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
            .find_map(|uri| image(uri, signal));
            ready.entries.push((entry.entry_id.to_string(), image));
        }
    }
    ready
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "popup_frozen_preview_tests.rs"]
mod tests;
