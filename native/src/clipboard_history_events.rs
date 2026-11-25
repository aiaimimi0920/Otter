//! History-menu event-subscription lifecycle; callbacks only coalesce invalidation signals, never read content or access windows.
use super::Apartment;
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};
use windows::{
    core::{IInspectable, Result},
    ApplicationModel::DataTransfer::{Clipboard, ClipboardHistoryChangedEventArgs},
    Foundation::EventHandler,
};

type Remove = fn(i64) -> Result<()>;

struct Tokens(Vec<(i64, Remove)>);
impl Drop for Tokens {
    fn drop(&mut self) {
        // Includes partial-registration failures; failure to unregister one subscription does not prevent trying the others.
        for (token, remove) in self.0.drain(..).rev() {
            let _ = remove(token);
        }
    }
}

fn invalidate(dirty: &Weak<AtomicBool>) {
    if let Some(dirty) = dirty.upgrade() {
        dirty.store(true, Ordering::Release);
    }
}

/// Create/destroy on the UI STA with a message pump; not movable across threads.
/// Start Task after reading the refresh flag; retain events arriving during the read for the next refresh.
pub struct Events {
    dirty: Arc<AtomicBool>,
    // Field order unregisters first and exits the apartment last; late callbacks capture only Weak.
    _tokens: Tokens,
    _apartment: Apartment,
    _thread: PhantomData<Rc<()>>,
}

impl Events {
    #[cfg(test)]
    pub(super) fn synthetic() -> Self {
        Self {
            dirty: Arc::new(AtomicBool::new(true)),
            _tokens: Tokens(Vec::new()),
            _apartment: Apartment::new().unwrap(),
            _thread: PhantomData,
        }
    }

    #[cfg(test)]
    pub(super) fn signal(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    pub fn subscribe() -> Result<Self> {
        let apartment = Apartment::new()?;
        let dirty = Arc::new(AtomicBool::new(true));
        let mut tokens = Tokens(Vec::with_capacity(3));
        let weak = Arc::downgrade(&dirty);
        let history = EventHandler::<ClipboardHistoryChangedEventArgs>::new(move |_, _| {
            invalidate(&weak);
            Ok(())
        });
        tokens.0.push((
            Clipboard::HistoryChanged(&history)?,
            Clipboard::RemoveHistoryChanged,
        ));
        let weak = Arc::downgrade(&dirty);
        let content = EventHandler::<IInspectable>::new(move |_, _| {
            invalidate(&weak);
            Ok(())
        });
        tokens.0.push((
            Clipboard::ContentChanged(&content)?,
            Clipboard::RemoveContentChanged,
        ));
        let weak = Arc::downgrade(&dirty);
        let enabled = EventHandler::<IInspectable>::new(move |_, _| {
            invalidate(&weak);
            Ok(())
        });
        tokens.0.push((
            Clipboard::HistoryEnabledChanged(&enabled)?,
            Clipboard::RemoveHistoryEnabledChanged,
        ));
        Ok(Self {
            dirty,
            _tokens: tokens,
            _apartment: apartment,
            _thread: PhantomData,
        })
    }

    /// Nonblocking coalesced notification; initially true, with no queue/history-content accumulation during event floods.
    pub fn take_changed(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }
}

#[cfg(test)]
#[path = "clipboard_history_events_tests.rs"]
mod tests;
