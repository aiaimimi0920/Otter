//! UI到原控制轮询线程的有界队列；取消优先于后续边沿，连续move合并，无新线程或句柄。
use crate::{live_input::Event, live_relay_http::input::Capabilities};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Status {
    pub available: bool,
    pub pending: bool,
    pub enabled: bool,
    pub failed: bool,
    pub recovering: bool,
}
#[derive(Default)]
struct State {
    capabilities: Capabilities,
    ready: bool,
    requested: bool,
    granted: bool,
    possible: bool,
    stopping: bool,
    failed: bool,
    releasing: bool,
    media_recovering: bool,
    control_recovering: bool,
    events: VecDeque<Event>,
    media: Weak<crate::live_relay_viewer_session::ViewerSession>,
}
impl State {
    fn recovering(&self) -> bool {
        self.media_recovering || self.control_recovering
    }
    fn release(&mut self) {
        self.requested = false;
        self.granted = false;
        self.releasing |= self.possible;
        self.events.clear();
    }
}
#[derive(Default)]
pub(crate) struct Queue {
    state: Mutex<State>,
    pub(crate) triggers: crate::live_relay_viewer_trigger_queue::Queue,
    pub(crate) local_triggers: crate::art_live_trigger::queue::Queue,
    #[cfg(test)]
    disconnect: Mutex<Option<crate::live_relay_viewer_runtime::Disconnect>>,
    #[cfg(test)]
    control: Mutex<Weak<crate::device_authorization_lease::Lease>>,
    #[cfg(test)]
    pub(crate) test_cursor: std::sync::atomic::AtomicU64,
}
impl Queue {
    #[cfg(test)]
    pub(crate) fn test_bind_disconnect(
        &self,
        disconnect: crate::live_relay_viewer_runtime::Disconnect,
    ) {
        *self.disconnect.lock().unwrap() = Some(disconnect);
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_media(&self) -> bool {
        self.disconnect
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|disconnect| disconnect.run())
    }
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub(in crate::live_relay_control_runtime) fn configure(
        &self,
        caps: Capabilities,
        media: &Arc<crate::live_relay_viewer_session::ViewerSession>,
    ) {
        let mut state = self.state();
        state.capabilities = caps;
        state.ready = true;
        state.media = Arc::downgrade(media);
    }
    fn notify(state: &State) {
        if let Some(media) = state.media.upgrade() {
            media.notify_input();
        }
    }
    pub(crate) fn notify_trigger(&self) {
        Self::notify(&self.state());
    }
    pub(crate) fn status(&self) -> Status {
        let s = self.state();
        let available = s.ready && !s.stopping && !s.recovering() && s.capabilities.available();
        Status {
            available,
            pending: available && s.requested && !s.granted,
            enabled: available && s.requested && s.granted,
            failed: s.failed,
            recovering: s.recovering(),
        }
    }
    pub(crate) fn request(&self) -> bool {
        let mut s = self.state();
        if !s.ready || s.stopping || s.recovering() || !s.capabilities.available() {
            return false;
        }
        s.failed = false;
        s.requested = true;
        Self::notify(&s);
        true
    }
    pub(crate) fn release(&self) {
        let mut s = self.state();
        s.release();
        Self::notify(&s);
    }
    /// 媒体丢失只减权；恢复后仍需用户显式获取，迟到acquire应答不恢复旧输入。
    pub(crate) fn media_recovering(&self, recovering: bool) {
        let mut s = self.state();
        s.media_recovering = recovering;
        self.local_triggers.pause(s.recovering());
        if recovering {
            s.release();
        }
        Self::notify(&s);
    }
    pub(crate) fn control_unavailable(&self) -> bool {
        self.state().control_recovering
    }
    pub(crate) fn control_recovering(&self, recovering: bool) {
        self.triggers.pause(recovering);
        let mut s = self.state();
        s.control_recovering = recovering;
        self.local_triggers.pause(s.recovering());
        if recovering {
            s.release();
        }
        Self::notify(&s);
    }
    #[cfg(test)]
    pub(crate) fn test_bind_control(
        &self,
        control: Weak<crate::device_authorization_lease::Lease>,
    ) {
        *self.control.lock().unwrap() = control;
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_control(&self) -> bool {
        self.control
            .lock()
            .unwrap()
            .upgrade()
            .is_some_and(|lease| lease.test_disconnect_request())
    }
    pub(super) fn retire_host(&self) {
        self.local_triggers.close();
        self.release();
        self.state().ready = false;
    }
    pub(crate) fn push(&self, event: Event) -> bool {
        let mut s = self.state();
        if s.stopping || !s.requested || !s.granted {
            return false;
        }
        if !s.capabilities.allows(event) || s.events.len() >= 16 {
            if s.capabilities.allows(event)
                && matches!(event, Event::Move { .. })
                && matches!(s.events.back(), Some(Event::Move { .. }))
            {
                *s.events.back_mut().unwrap() = event;
                return true;
            }
            s.failed = true;
            s.requested = false;
            s.granted = false;
            s.releasing |= s.possible;
            s.events.clear();
            Self::notify(&s);
            return false;
        }
        if matches!(event, Event::Move { .. })
            && matches!(s.events.back(), Some(Event::Move { .. }))
        {
            *s.events.back_mut().unwrap() = event;
        } else {
            s.events.push_back(event);
        }
        Self::notify(&s);
        true
    }
    pub(crate) fn stop(&self) {
        self.local_triggers.close();
        self.triggers.stop();
        self.release();
        let mut s = self.state();
        s.stopping = true;
        Self::notify(&s);
    }
    pub(in crate::live_relay_control_runtime) fn cleanup_needed(&self) -> bool {
        self.state().possible
    }
    pub(in crate::live_relay_control_runtime) fn intent(&self) -> (bool, bool) {
        let s = self.state();
        (s.requested && !s.stopping, s.stopping)
    }
    pub(super) fn releasing(&self) -> bool {
        self.state().releasing
    }
    pub(in crate::live_relay_control_runtime) fn mark_possible(&self) {
        let mut s = self.state();
        s.possible = true;
        s.releasing |= !s.requested || s.stopping;
    }
    pub(in crate::live_relay_control_runtime) fn acknowledge(&self, owned: bool) {
        let mut s = self.state();
        s.possible = owned;
        if !owned {
            s.releasing = false;
        }
        s.granted = owned && s.requested && !s.stopping && !s.releasing;
        Self::notify(&s);
    }
    pub(in crate::live_relay_control_runtime) fn take(&self) -> Option<Event> {
        let mut s = self.state();
        if s.requested && !s.stopping && !s.releasing && s.granted {
            s.events.pop_front()
        } else {
            None
        }
    }
    pub(in crate::live_relay_control_runtime) fn finish(&self, failed: bool) {
        self.local_triggers.close();
        self.triggers.stop();
        let mut s = self.state();
        s.events.clear();
        s.requested = false;
        s.granted = false;
        s.possible = false;
        s.stopping = true;
        s.failed |= failed;
        Self::notify(&s);
    }
}
#[cfg(test)]
#[path = "live_relay_viewer_input_queue_tests.rs"]
mod tests;
