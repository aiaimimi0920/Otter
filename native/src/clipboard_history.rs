//! Windows history snapshots and explicit single-item deletion; background STA only, with no persistence or writes to the current clipboard.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows::{
    core::{Error, Result},
    ApplicationModel::DataTransfer::{Clipboard, ClipboardHistoryItemsResultStatus},
    Win32::{
        Foundation::E_ABORT,
        System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED},
    },
};
#[path = "clipboard_history_content.rs"]
mod content;
#[path = "clipboard_history_events.rs"]
pub mod events;
#[path = "clipboard_history_preview.rs"]
mod preview;
#[path = "clipboard_history_remove.rs"]
mod remove;
#[path = "clipboard_history_session.rs"]
pub mod session;
#[path = "clipboard_history_task.rs"]
pub mod task;
#[path = "clipboard_history_wait.rs"]
mod wait;

pub const MAX_ITEMS: u32 = 25;
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
static READING: AtomicBool = AtomicBool::new(false);
struct ReadPermit;
impl ReadPermit {
    fn acquire() -> Result<Self> {
        READING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| Error::new(E_ABORT, "Clipboard history read already active"))?;
        Ok(Self)
    }
}
impl Drop for ReadPermit {
    fn drop(&mut self) {
        READING.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Disabled,
    Denied,
}
pub enum Content {
    Bitmap {
        preview: crate::image_decode::Image,
        thumbnail: crate::image_thumbnail::Thumbnail,
        encoded: crate::pixels::Pixels,
        width: u32,
        height: u32,
    },
    Text(String),
}
pub struct Entry {
    pub id: String,
    pub content: Content,
}
pub struct Snapshot {
    pub status: Status,
    pub entries: Vec<Entry>,
}

struct Apartment;
impl Apartment {
    fn new() -> Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_SINGLETHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

pub(super) fn check(cancel: &AtomicBool, deadline: Instant) -> Result<()> {
    if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
        Err(Error::new(
            E_ABORT,
            "Clipboard history read cancelled or timed out",
        ))
    } else {
        Ok(())
    }
}

/// Call on a dedicated background thread; process-wide permits prevent blocking providers from accumulating concurrent reads.
/// Synchronous WinRT calls themselves have no hard-timeout guarantee; retain the permit until the read truly returns.
pub fn read(cancel: &AtomicBool) -> Result<Snapshot> {
    check(cancel, Instant::now() + Duration::from_secs(5))?;
    read_with_permit(cancel, ReadPermit::acquire()?)
}

fn read_with_permit(cancel: &AtomicBool, _permit: ReadPermit) -> Result<Snapshot> {
    let deadline = Instant::now() + Duration::from_secs(5);
    check(cancel, deadline)?;
    let _apartment = Apartment::new()?;
    let result = wait::operation!(Clipboard::GetHistoryItemsAsync()?, cancel, deadline)?;
    let status = result.Status()?;
    if status != ClipboardHistoryItemsResultStatus::Success {
        return Ok(Snapshot {
            status: if status == ClipboardHistoryItemsResultStatus::ClipboardHistoryDisabled {
                Status::Disabled
            } else {
                Status::Denied
            },
            entries: Vec::new(),
        });
    }
    let items = result.Items()?;
    let mut entries = Vec::new();
    let mut remaining = MAX_TOTAL_BYTES;
    for index in 0..items.Size()?.min(MAX_ITEMS) {
        check(cancel, deadline)?;
        // Expired/unsupported entries do not hide other history; cancellation still propagates at the next boundary.
        let entry = (|| {
            let item = items.GetAt(index)?;
            let id = item.Id()?;
            if id.len() > 1024 {
                return Ok(None);
            }
            content::decode(&item.Content()?, remaining, cancel, deadline).map(|value| {
                value.map(|content| Entry {
                    id: id.to_string(),
                    content,
                })
            })
        })();
        match entry {
            Err(error) if error.code() == windows::Win32::Foundation::E_OUTOFMEMORY => {
                return Err(error)
            }
            Ok(Some(entry)) => {
                remaining -= match &entry.content {
                    Content::Bitmap {
                        encoded,
                        thumbnail,
                        preview,
                        ..
                    } => encoded.len() + thumbnail.pixels().len() + preview.pixels.len(),
                    Content::Text(text) => text.len(),
                };
                entries.push(entry);
            }
            _ => {}
        }
    }
    check(cancel, deadline)?;
    Ok(Snapshot {
        status: Status::Ready,
        entries,
    })
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "clipboard_history_tests.rs"]
mod tests;
