//! Window-scoped decoded pixels. Disk/save snapshots never own them; a reopened panel
//! has a new generation and retries failures without retrying on every message-pump tick.
use crate::capture_history::{image, Data, Screenshot};
use std::sync::{Arc, Weak};

#[derive(Default)]
pub(in crate::desktop) struct Previews {
    generation: Option<Arc<()>>,
    entries: Vec<(Arc<()>, Option<image::Preview>)>,
    pub revision: u64,
}
pub(in crate::desktop) struct Batch {
    generation: Weak<()>,
    data: Arc<Data>,
    missing: Vec<usize>,
}
pub(in crate::desktop) struct Decoded {
    generation: Weak<()>,
    entries: Vec<(Arc<()>, Option<image::Preview>)>,
}
impl Previews {
    pub fn open(&mut self) {
        self.generation = Some(Arc::new(()));
        self.entries.clear();
    }
    pub fn close(&mut self) {
        self.generation = None;
        self.entries.clear();
    }
    pub fn get(&self, shot: &Screenshot) -> Option<&image::Preview> {
        self.entries
            .iter()
            .find(|(k, _)| Arc::ptr_eq(k, &shot.key))?
            .1
            .as_ref()
    }
    pub fn failed(&self) -> bool {
        self.entries.iter().any(|(_, p)| p.is_none())
    }
    pub fn retain_current(&mut self, data: &Data) {
        self.entries
            .retain(|(key, _)| data.screenshots.iter().any(|s| Arc::ptr_eq(key, &s.key)));
    }
    pub fn captured(&mut self, entry: &mut Screenshot) {
        if let Some(preview) = entry.preview.take() {
            if self.generation.is_some() {
                self.entries.push((entry.key.clone(), Some(preview)));
            }
        }
    }
    pub fn batch(&self, data: &Arc<Data>) -> Option<Batch> {
        let generation = self.generation.as_ref()?;
        if self.entries.len() == data.screenshots.len() {
            return None;
        }
        let missing: Vec<_> = data
            .screenshots
            .iter()
            .enumerate()
            .filter(|(_, shot)| !self.entries.iter().any(|(k, _)| Arc::ptr_eq(k, &shot.key)))
            .map(|(i, _)| i)
            .collect();
        (!missing.is_empty()).then(|| Batch {
            generation: Arc::downgrade(generation),
            data: data.clone(),
            missing,
        })
    }
    pub fn publish(&mut self, decoded: Decoded, data: &Data) {
        if !self
            .generation
            .as_ref()
            .is_some_and(|g| decoded.generation.ptr_eq(&Arc::downgrade(g)))
        {
            return;
        }
        for (key, preview) in decoded.entries {
            if data
                .screenshots
                .iter()
                .any(|shot| Arc::ptr_eq(&shot.key, &key))
            {
                self.entries.push((key, preview));
            }
        }
        self.revision = self.revision.wrapping_add(1);
    }
}
impl Batch {
    pub fn on_failure(&self) -> Decoded {
        Decoded {
            generation: self.generation.clone(),
            entries: self
                .missing
                .iter()
                .map(|&i| (self.data.screenshots[i].key.clone(), None))
                .collect(),
        }
    }
    pub fn decode(self) -> Decoded {
        let com = crate::desktop::Apartment::new();
        self.decode_with(|url| {
            com.as_ref()
                .ok()
                .and_then(|_| image::decode_preview(url).ok())
        })
    }
    pub(super) fn decode_with(
        self,
        mut decode: impl FnMut(&str) -> Option<image::Preview>,
    ) -> Decoded {
        let mut entries = Vec::new();
        for i in self.missing {
            if self.generation.strong_count() == 0 {
                break;
            }
            let shot = &self.data.screenshots[i];
            entries.push((shot.key.clone(), decode(&shot.thumbnail)));
        }
        Decoded {
            generation: self.generation,
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::capture_history_ui::View;
    #[test]
    fn full_panel_retries_only_on_reopen_rejects_stale_pixels_and_releases_all_leases() {
        let _com = crate::desktop::Apartment::new().unwrap();
        let source = crate::image_decode::Image {
            width: 240,
            height: 240,
            pixels: [20, 50, 100, 255].repeat(240 * 240).into(),
        };
        let mut view = View::default();
        for i in 0..crate::capture_history::LIMIT {
            view.screenshot(image::capture(i.to_string(), &source, i as u64).unwrap());
        }
        let saved = view.data.clone();
        assert!(saved.screenshots.iter().all(|s| s.preview.is_none()));
        assert!(view.previews.batch(&saved).is_none());
        view.previews.open();
        let failed = view.previews.batch(&saved).unwrap().decode_with(|_| None);
        view.previews.publish(failed, &saved);
        assert!(view.previews.failed());
        assert!(view.previews.batch(&saved).is_none());
        view.previews.close();
        view.previews.open();
        let old = view.previews.batch(&saved).unwrap().decode();
        view.previews.close();
        view.previews.open();
        view.previews.publish(old, &saved);
        assert!(
            view.previews.entries.is_empty(),
            "no resurrection after close/reopen"
        );
        let current = view.previews.batch(&saved).unwrap().decode();
        let first = &saved.screenshots[0];
        view.screenshot(image::capture(first.id.clone(), &source, first.at).unwrap());
        view.previews.publish(current, &view.data);
        assert!(
            view.previews.get(first).is_none(),
            "same id and timestamp, different capture"
        );
        assert_eq!(view.previews.entries.len(), crate::capture_history::LIMIT);
        assert!(!view.previews.failed());
        let leases: Vec<_> = saved.screenshots[1..]
            .iter()
            .map(|s| view.previews.get(s).unwrap().image.pixels.downgrade())
            .collect();
        assert!(leases.iter().all(|p| p.upgrade().is_some()));
        view.previews.close();
        assert!(
            leases.iter().all(|p| p.upgrade().is_none()),
            "save and window snapshots cannot retain decoded pixels"
        );
        assert_eq!(saved.screenshots.len(), 24);
    }
}
