//! Single-slot frame candidates/actual presentation receipts; network threads hold no HWNDs, and window threads cannot extend six-second service authorization.
use crate::{
    image_decode::Image,
    tile_clock::Clock,
    tile_wire::{Layout, Size},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
#[path = "tile_presenter_admission.rs"]
pub(crate) mod admission;
#[path = "tile_presentation_trace.rs"]
pub(crate) mod diagnostics;
/// Workers retain locale IDs, while protocol/decoder diagnostics remain verbatim.
/// Resolve at the UI boundary so cached states follow the current display language.
pub(crate) fn display_status(code: &'static str) -> &'static str {
    display_status_for(crate::ui_text::current_language(), code)
}
fn display_status_for(
    language: crate::ui_text::DisplayLanguage,
    code: &'static str,
) -> &'static str {
    if code.starts_with("tile-") || code.starts_with("integration-") {
        crate::ui_text::text_for(language, code)
    } else {
        code
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    pub endpoint: String,
    pub lease: String,
    pub wall: String,
    pub revision: u64,
    pub clock: String,
}

#[cfg(test)]
mod locale_tests {
    use super::*;
    use crate::ui_text::DisplayLanguage;

    #[test]
    fn retained_status_is_resolved_for_each_display_language() {
        let state = State::default();
        assert_eq!(state.status, "tile-wait-authorization");
        assert_eq!(
            display_status_for(DisplayLanguage::English, state.status),
            "Waiting for device authorization"
        );
        assert_eq!(
            display_status_for(DisplayLanguage::SimplifiedChinese, state.status),
            "等待设备授权"
        );
        assert_eq!(
            display_status_for(DisplayLanguage::English, "tile_surface_invalid"),
            "tile_surface_invalid"
        );
        assert_eq!(display_status_for(DisplayLanguage::English, ""), "");
    }
}
pub(crate) struct Frame {
    pub key: Key,
    pub layout: Layout,
    pub size: Size,
    pub images: BTreeMap<String, Image>,
    pub arts: BTreeMap<String, Arc<crate::tile_surface_owner::Art>>,
    pub live_until: Option<Instant>,
}
/// A borrowed candidate; build the owned frame only after admission and deduplication.
pub(crate) struct Candidate<'a, I> {
    pub key: &'a Key,
    pub layout: &'a Layout,
    pub size: Size,
    // Static and live image IDs are disjoint after the asset worker's source-kind check.
    pub images: I,
    pub arts: &'a BTreeMap<String, Arc<crate::tile_surface_owner::Art>>,
    pub until: Option<Instant>,
}
pub(crate) struct State {
    pub stopped: bool,
    pub generation: u64,
    pub desired: Option<Key>,
    pub frame: Option<Arc<Frame>>,
    pub presented: Option<(Key, u64, u64)>,
    pub presented_frame: Option<Arc<Frame>>,
    pub input_applied: Option<admission::Acknowledged>,
    pub until: Option<Instant>,
    pub clock: Clock,
    pub activate_at: u64,
    pub frozen: bool,
    pub black: bool,
    /// Keep locale IDs rather than translated text so a language switch cannot stale the status.
    pub status: &'static str,
    pub black_revision: Option<u64>,
    pub black_confirmed: Option<u64>,
    pub expired: bool,
    pub art: Option<Arc<crate::tile_surface_worker::Prepared>>,
    pub identification: crate::tile_identification::Identification,
    pub actions: crate::tile_surface_actions::Queue,
}
impl Default for State {
    fn default() -> Self {
        Self {
            stopped: false,
            generation: 1,
            desired: None,
            frame: None,
            presented: None,
            presented_frame: None,
            input_applied: None,
            until: None,
            clock: Clock::default(),
            activate_at: 0,
            frozen: false,
            black: true,
            status: "tile-wait-authorization",
            black_revision: None,
            black_confirmed: None,
            expired: false,
            art: None,
            identification: Default::default(),
            actions: Default::default(),
        }
    }
}
impl State {
    pub(crate) fn unavailable(&mut self, generation: u64, key: &Key, reason: &'static str) {
        if !self.stopped
            && !self.frozen
            && !self.black
            && self.generation == generation
            && self.desired.as_ref() == Some(key)
        {
            self.frame = None;
            self.presented = None;
            self.presented_frame = None;
            self.status = reason;
        }
    }
    pub fn clear(&mut self, reason: &'static str) {
        self.actions.clear();
        self.identification.clear();
        self.generation = self.generation.saturating_add(1);
        self.desired = None;
        self.frame = None;
        self.presented = None;
        self.presented_frame = None;
        self.input_applied = None;
        self.until = None;
        self.frozen = false;
        self.black = true;
        self.status = reason;
        self.black_revision = None;
        self.black_confirmed = None;
        self.expired = false;
        self.art = None;
    }
    pub fn expire(&mut self, now: Instant) {
        self.identification.expire(now);
        // Media expiry clears only candidates/receipts, retaining desired/generation so original tasks can keep preparing; freeze preserves the actually presented snapshot.
        if !self.frozen {
            if self
                .frame
                .as_ref()
                .is_some_and(|f| f.live_until.is_some_and(|v| now > v))
            {
                self.frame = None;
            }
            if self
                .presented_frame
                .as_ref()
                .is_some_and(|f| f.live_until.is_some_and(|v| now > v))
            {
                self.presented = None;
                self.presented_frame = None;
            }
        }
        if !self.expired && self.until.is_some_and(|until| now >= until) {
            self.expired = true;
            self.generation = self.generation.saturating_add(1);
            self.art = None;
            self.identification.clear();
            self.actions.clear();
            self.frame = None;
            self.presented = None;
            self.presented_frame = None;
            self.input_applied = None;
            self.black_confirmed = None;
            self.status = "tile-authorization-expired";
        }
    }
    pub fn visible(&self, now: Instant) -> bool {
        self.visibility(now) == diagnostics::Stage::Visible
    }
    pub(crate) fn visibility(&self, now: Instant) -> diagnostics::Stage {
        diagnostics::visibility(self, now)
    }
}
#[derive(Default)]
pub(crate) struct Shared {
    pub state: Mutex<State>,
    pub trace: diagnostics::Trace,
    wake: Condvar,
}
impl Shared {
    pub fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn stop(&self) {
        let mut s = self.lock();
        s.stopped = true;
        s.clear("tile-stopped");
        self.wake.notify_all();
    }
    pub fn wait(&self, time: Duration) -> bool {
        let guard = self.lock();
        if guard.stopped {
            return false;
        }
        !self
            .wake
            .wait_timeout(guard, time)
            .unwrap_or_else(|e| e.into_inner())
            .0
            .stopped
    }
    #[cfg(test)]
    pub fn publish(&self, generation: u64, frame: Frame) {
        self.publish_until(generation, frame, None);
    }
    pub fn unavailable(&self, generation: u64, key: &Key, reason: &'static str) {
        self.lock().unavailable(generation, key, reason);
    }
    #[cfg(test)]
    pub fn publish_until(&self, generation: u64, frame: Frame, until: Option<Instant>) {
        self.publish_prepared(generation, frame, until, None);
    }
    #[cfg(test)]
    pub fn publish_prepared(
        &self,
        generation: u64,
        frame: Frame,
        until: Option<Instant>,
        art: Option<&Arc<crate::tile_surface_worker::Prepared>>,
    ) {
        self.publish_candidate(
            generation,
            Candidate {
                key: &frame.key,
                layout: &frame.layout,
                size: frame.size,
                images: frame.images.iter(),
                arts: &frame.arts,
                until,
            },
            art,
        );
    }
    pub fn publish_candidate<'a>(
        &self,
        generation: u64,
        candidate: Candidate<'a, impl Iterator<Item = (&'a String, &'a Image)> + Clone>,
        art: Option<&Arc<crate::tile_surface_worker::Prepared>>,
    ) {
        let mut s = self.lock();
        if s.stopped
            || s.frozen
            || s.black
            || s.generation != generation
            || s.desired.as_ref() != Some(candidate.key)
            || match (&s.art, art) {
                (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                (None, None) => false,
                _ => true,
            }
        {
            return;
        }
        let same = s.frame.as_ref().is_some_and(|old| {
            &old.key == candidate.key
                && old.size == candidate.size
                && old.arts.len() == candidate.arts.len()
                && old
                    .arts
                    .iter()
                    .all(|(id, a)| candidate.arts.get(id).is_some_and(|b| Arc::ptr_eq(a, b)))
                && old.images.len() == candidate.images.clone().count()
                && candidate.images.clone().all(|(id, a)| {
                    old.images.get(id).is_some_and(|b| {
                        a.width == b.width
                            && a.height == b.height
                            && a.pixels.same_storage(&b.pixels)
                    })
                })
        });
        if !same {
            let previous = s.frame.replace(Arc::new(Frame {
                key: candidate.key.clone(),
                layout: candidate.layout.clone(),
                size: candidate.size,
                images: candidate
                    .images
                    .map(|(id, image)| (id.clone(), image.clone()))
                    .collect(),
                arts: candidate.arts.clone(),
                live_until: candidate.until,
            }));
            s.status = "tile-content-ready";
            drop(s);
            drop(previous);
        }
    }
    pub fn confirm(&self, frame: &Arc<Frame>, now: Instant) {
        use diagnostics::Stage;
        let mut s = self.lock();
        let visibility = s.visibility(now);
        let stage = if visibility != Stage::Visible {
            visibility
        } else if s.desired.as_ref() != Some(&frame.key) {
            Stage::KeyChangedDuringPaint
        } else if !s.frozen && frame.live_until.is_some_and(|v| now > v) {
            Stage::MediaExpiredDuringPaint
        } else if !s.frame.as_ref().is_some_and(|current| {
            current.arts.len() == frame.arts.len()
                && current
                    .arts
                    .iter()
                    .all(|(id, art)| frame.arts.get(id).is_some_and(|a| Arc::ptr_eq(a, art)))
        }) {
            Stage::ArtChangedDuringPaint
        } else {
            // Background work may publish the next frame during paint; freeze preserves only this complete snapshot that actually finished presentation.
            s.presented_frame = Some(frame.clone());
            if let Some((time, error)) = s.clock.estimate(now) {
                if s.presented.as_ref().is_none_or(|p| p.0 != frame.key) {
                    s.presented = Some((frame.key.clone(), time, error));
                }
                s.status = "tile-presented";
                Stage::Confirmed
            } else {
                Stage::ClockUnavailable
            }
        };
        drop(s);
        self.trace.observe(stage);
    }
    pub fn confirm_black(&self, now: Instant) {
        let mut s = self.lock();
        if !s.stopped && s.black && s.until.is_some_and(|v| now < v) {
            s.black_confirmed = s.black_revision;
        }
    }
}
