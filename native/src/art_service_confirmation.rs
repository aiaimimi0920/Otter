//! 单在途宿主确认槽；UI只能交回原票据，期限/撤销/关闭使其不可逆失效。
use super::Error;
use std::sync::{Arc, Mutex};
use std::time::Instant;
#[path = "art_service_confirmation_preview.rs"]
pub(super) mod preview;
pub(crate) struct Preview {
    pub label: String,
    pub size: [u32; 2],
    pub image: crate::image_thumbnail::Thumbnail,
}

pub(crate) struct Ticket {
    pub text: String,
    pub images: Vec<Preview>,
    deadline: Instant,
    state: Mutex<State>,
    notify: Arc<dyn Fn() + Send + Sync>,
}
struct State {
    live: bool,
    choice: Option<bool>,
}
impl Ticket {
    pub fn live(&self) -> bool {
        Instant::now() < self.deadline
            && self
                .state
                .lock()
                .is_ok_and(|state| state.live && state.choice.is_none())
    }
    pub fn resolve(&self, approve: bool) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if !state.live || state.choice.is_some() || Instant::now() >= self.deadline {
            return false;
        }
        state.choice = Some(approve);
        drop(state);
        self.wake();
        true
    }
    pub fn choice(&self) -> Option<bool> {
        self.state.lock().ok().filter(|state| state.live)?.choice
    }
    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.live = false;
            state.choice = None;
        }
        self.wake();
    }
    pub(crate) fn wake(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.notify)()));
    }
}
#[derive(Default)]
struct Slot {
    closed: bool,
    active: Option<Arc<Ticket>>,
    delivered: bool,
}
pub(crate) struct Channel {
    slot: Mutex<Slot>,
    notify: Arc<dyn Fn() + Send + Sync>,
}
impl Channel {
    pub fn new(notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            slot: Mutex::new(Slot::default()),
            notify,
        }
    }
    #[cfg(test)]
    pub fn publish(&self, text: String, deadline: Instant) -> Result<Arc<Ticket>, Error> {
        self.publish_images(text, deadline, Vec::new())
    }
    pub(super) fn publish_images(
        &self,
        text: String,
        deadline: Instant,
        images: Vec<Preview>,
    ) -> Result<Arc<Ticket>, Error> {
        if images.len() > 4 || text.len() > 512 * 1024 {
            return Err(Error::Limit);
        }
        let mut slot = self.slot.lock().map_err(|_| Error::Stopped)?;
        if slot.closed || slot.active.is_some() || Instant::now() >= deadline {
            return Err(Error::Stopped);
        }
        let ticket = Arc::new(Ticket {
            text,
            images,
            deadline,
            state: Mutex::new(State {
                live: true,
                choice: None,
            }),
            notify: self.notify.clone(),
        });
        slot.active = Some(ticket.clone());
        slot.delivered = false;
        drop(slot);
        ticket.wake();
        Ok(ticket)
    }
    pub fn take(&self) -> Option<Arc<Ticket>> {
        let mut slot = self.slot.lock().ok()?;
        if slot.closed || slot.delivered {
            return None;
        }
        slot.delivered = true;
        slot.active.as_ref().filter(|value| value.live()).cloned()
    }
    pub fn retire(&self, ticket: &Arc<Ticket>) {
        ticket.close();
        if let Ok(mut slot) = self.slot.lock() {
            if slot
                .active
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, ticket))
            {
                slot.active = None;
                slot.delivered = false;
            }
        }
    }
    pub fn close(&self) {
        let ticket = self.slot.lock().ok().and_then(|mut slot| {
            slot.closed = true;
            slot.active.take()
        });
        if let Some(ticket) = ticket {
            ticket.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn confirmation_slot_is_bounded_single_use_and_old_ticket_cannot_approve_new_call() {
        let channel = Channel::new(Arc::new(|| {}));
        let first = channel
            .publish("first".into(), Instant::now() + Duration::from_secs(1))
            .unwrap();
        assert!(channel
            .publish("overflow".into(), Instant::now() + Duration::from_secs(1))
            .is_err());
        assert!(Arc::ptr_eq(&first, &channel.take().unwrap()));
        assert!(channel.take().is_none());
        channel.retire(&first);
        assert!(!first.resolve(true));
        let second = channel
            .publish("second".into(), Instant::now() + Duration::from_secs(1))
            .unwrap();
        channel.retire(&first);
        assert!(second.live());
        assert!(second.resolve(false));
        assert!(!second.resolve(true));
        assert_eq!(second.choice(), Some(false));
        channel.close();
        assert_eq!(second.choice(), None);
        assert!(channel
            .publish("closed".into(), Instant::now() + Duration::from_secs(1))
            .is_err());
    }
    #[test]
    fn expired_confirmation_cannot_be_approved_or_delivered() {
        let channel = Channel::new(Arc::new(|| {}));
        assert!(channel.publish("expired".into(), Instant::now()).is_err());
    }
}
