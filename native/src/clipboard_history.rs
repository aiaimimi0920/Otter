//! 只读 Windows 历史快照；仅后台 STA 调用，内容不落盘、不记录日志、不写系统剪贴板。
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
#[path = "clipboard_history_list.rs"]
pub mod list;
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

/// 调用方必须在专属后台线程执行；进程级许可防止阻塞提供方积累并发读取。
/// WinRT 同步调用自身不受硬超时保证，许可必须保持到该读取真正返回。
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
        // 过期或不支持的条目不遮蔽其余历史；取消仍在下一边界传播。
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
        if let Ok(Some(entry)) = entry {
            remaining -= match &entry.content {
                Content::Bitmap {
                    encoded, thumbnail, ..
                } => encoded.len() + thumbnail.pixels().len(),
                Content::Text(text) => text.len(),
            };
            entries.push(entry);
        }
    }
    check(cancel, deadline)?;
    Ok(Snapshot {
        status: Status::Ready,
        entries,
    })
}

#[cfg(test)]
#[path = "clipboard_history_tests.rs"]
mod tests;
