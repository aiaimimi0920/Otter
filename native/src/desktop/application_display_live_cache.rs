//! Live 首次/显式刷新共用显示单槽；每个 owner 只排队最新固定帧，不逐帧 CPU 读回。
use super::*;
type Frame = Arc<crate::live_frames::Frame<crate::live_texture::Texture>>;
pub(super) struct Refresh {
    source: UnitId,
    identity: Arc<()>,
    frame: Frame,
}
impl Cache {
    // 只在成功读回后替换缓存并排队通知；失败保留上一张有效画面。
    pub(super) fn accept_live_result(&mut self, key: &Key, success: bool) -> bool {
        let Origin::Live(identity) = &key.origin else {
            return true;
        };
        if !success
            && self.entries.iter().any(|entry| {
                entry.key.source == key.source
                    && matches!(&entry.key.origin, Origin::Live(old) if Arc::ptr_eq(old, identity))
                    && entry.result.is_ok()
            })
        {
            return false;
        }
        self.entries.retain(|entry| {
            !(entry.key.source == key.source && matches!(&entry.key.origin, Origin::Live(_)))
        });
        if success {
            self.live_updates
                .retain(|(source, _)| source != &key.source);
            self.live_updates
                .push((key.source.clone(), identity.clone()));
        }
        true
    }
    pub(in crate::desktop::application) fn take_live_updates(&mut self) -> Vec<(UnitId, Arc<()>)> {
        std::mem::take(&mut self.live_updates)
    }
    pub(in crate::desktop::application) fn refresh_live(
        &mut self,
        source: UnitId,
        identity: Arc<()>,
        frame: Frame,
    ) -> Result<(), ()> {
        self.live_updates.retain(|(unit, _)| unit != &source);
        self.live_refreshes
            .retain(|pending| pending.source != source);
        if self.live_refreshes.len() >= crate::live_job::MAX_SESSIONS {
            return Err(());
        }
        if let Some(running) = &self.running {
            if running.key.source == source && matches!(&running.key.origin, Origin::Live(_)) {
                running.cancel.store(true, Ordering::Release);
            }
        }
        // 每个 owner 仅保留最新固定帧，整个集合受 Live 会话上限约束。
        self.live_refreshes.push(Refresh {
            source,
            identity,
            frame,
        });
        Ok(())
    }
    pub(in crate::desktop::application) fn retain_live(
        &mut self,
        mut current: impl FnMut(&UnitId, &Arc<()>) -> bool,
    ) -> bool {
        self.live_refreshes
            .retain(|pending| current(&pending.source, &pending.identity));
        self.live_updates
            .retain(|(source, identity)| current(source, identity));
        let mut keep = |key: &Key| match &key.origin {
            Origin::Live(identity) => current(&key.source, identity),
            Origin::Document { .. } => true,
        };
        let before = self.entries.len();
        self.entries.retain(|entry| keep(&entry.key));
        let mut changed = before != self.entries.len();
        if let Some(running) = &self.running {
            if !keep(&running.key) {
                changed |= !running.cancel.swap(true, Ordering::AcqRel);
            }
        }
        changed
    }
    pub(in crate::desktop::application) fn request_live(
        &mut self,
        source: &UnitId,
        identity: Arc<()>,
        capture: impl FnOnce() -> Result<Option<Frame>, ()>,
        notify: Notify,
    ) -> Result<Option<Image>, ()> {
        if self.running.is_none() {
            if let Some(index) = self.live_refreshes.iter().position(|pending| {
                pending.source == *source && Arc::ptr_eq(&pending.identity, &identity)
            }) {
                let pending = self.live_refreshes.remove(index);
                let _ = self.start_live(
                    Key {
                        source: pending.source,
                        origin: Origin::Live(pending.identity),
                    },
                    pending.frame,
                    notify.clone(),
                );
            }
        }
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.key.source == *source
                && matches!(&entry.key.origin, Origin::Live(old) if Arc::ptr_eq(old, &identity))
        }) {
            return entry.result.clone().map(Some);
        }
        if self.running.is_some() {
            return Ok(None);
        }
        let key = Key {
            source: source.clone(),
            origin: Origin::Live(identity),
        };
        let frame = match capture() {
            Ok(Some(frame)) => frame,
            Ok(None) => return Ok(None),
            Err(()) => {
                self.entries.push(Entry {
                    key,
                    result: Err(()),
                });
                return Err(());
            }
        };
        self.start_live(key, frame, notify)
    }
    fn start_live(&mut self, key: Key, frame: Frame, notify: Notify) -> Result<Option<Image>, ()> {
        self.start(key, notify, move |cancel| {
            let image = crate::live_texture::readback::read(frame, cancel).map_err(|error| {
                eprintln!("Live display readback failed: {error:?}");
            })?;
            Ok(Image {
                width: image.width,
                height: image.height,
                pixels: image.pixels,
            })
        })
    }
}
