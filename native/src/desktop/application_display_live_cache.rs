//! Initial/explicit Live refresh shares one display slot; queue only the latest fixed frame per owner, with no per-frame CPU readback.
use super::*;
type Frame = Arc<crate::live_frames::Frame<crate::live_texture::Texture>>;
pub(super) struct Refresh {
    source: UnitId,
    identity: Arc<()>,
    frame: Frame,
}
impl Cache {
    // Replace the cache and queue notifications only after successful readback; failure preserves the last valid image.
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
        // Retain only the latest fixed frame per owner; the Live session limit bounds the entire collection.
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
    fn live_entry(&self, source: &UnitId, identity: &Arc<()>) -> Option<&Entry> {
        self.entries.iter().find(|entry| {
            entry.key.source == *source
                && matches!(&entry.key.origin, Origin::Live(old) if Arc::ptr_eq(old, identity))
        })
    }
    pub(in crate::desktop::application) fn live_image(
        &self,
        source: &UnitId,
        identity: &Arc<()>,
    ) -> Option<&Image> {
        self.live_entry(source, identity)?.result.as_ref().ok()
    }
    /// true means the first image is still pending; the old committed image remains available to the display graph during refresh.
    pub(in crate::desktop::application) fn poll_live_source(
        &mut self,
        source: &UnitId,
        identity: Arc<()>,
        capture: impl FnOnce() -> Result<Option<Frame>, ()>,
        notify: impl Fn() -> Notify,
    ) -> bool {
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
                    notify(),
                );
            }
        }
        if self.live_entry(source, &identity).is_some() {
            return false;
        }
        if self.running.is_some() {
            return true;
        }
        let key = Key {
            source: source.clone(),
            origin: Origin::Live(identity),
        };
        let frame = match capture() {
            Ok(Some(frame)) => frame,
            Ok(None) => return true,
            Err(()) => {
                self.entries.push(Entry {
                    key,
                    result: Err(()),
                });
                return false;
            }
        };
        self.start_live(key, frame, notify()).is_ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settled_live_poll_neither_captures_nor_builds_a_notifier() {
        let source = UnitId::parse("live").unwrap();
        let owner = Arc::new(());
        let pixels = vec![0, 0, 0, 255].into();
        let image = Image {
            width: 1,
            height: 1,
            pixels,
        };
        let mut cache = Cache::default();
        cache.entries.push(Entry {
            key: Key {
                source: source.clone(),
                origin: Origin::Live(owner.clone()),
            },
            result: Ok(image.clone()),
        });
        assert!(!cache.poll_live_source(
            &source,
            owner.clone(),
            || panic!("settled source must not capture another frame"),
            || panic!("settled source must not build a notifier"),
        ));
        assert!(cache
            .live_image(&source, &owner)
            .unwrap()
            .pixels
            .same_storage(&image.pixels));
        assert!(cache.live_image(&source, &Arc::new(())).is_none());
        assert!(cache.retain_live(|_, _| false));
        assert!(cache.live_image(&source, &owner).is_none());
    }
}
