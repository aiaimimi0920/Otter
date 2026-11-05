//! 源窗口销毁的 UI 线程事件兜底；不依赖 WGC Closed 必达，不建立轮询或额外线程。
use super::Identity;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::Arc,
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};
struct Entry {
    identity: Identity,
    fired: Cell<bool>,
    notify: Arc<dyn Fn() + Send + Sync>,
}
thread_local! {
    static ENTRIES: RefCell<Vec<(isize, Weak<Entry>)>> = const { RefCell::new(Vec::new()) };
}
pub(crate) struct Watcher {
    hook: HWINEVENTHOOK,
    _entry: Rc<Entry>,
}
impl Watcher {
    pub(super) fn install(identity: Identity, notify: Arc<dyn Fn() + Send + Sync>) -> Result<Self> {
        ENTRIES.with(|entries| -> Result<()> {
            let mut entries = entries.borrow_mut();
            if entries.len() >= crate::live_job::MAX_SESSIONS {
                return Err(Error::new(
                    E_OUTOFMEMORY,
                    "Live destroy watcher limit reached",
                ));
            }
            entries
                .try_reserve(1)
                .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate Live watcher"))
        })?;
        let hook = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_DESTROY,
                None,
                Some(on_event),
                identity.process,
                identity.thread,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if hook.is_invalid() {
            return Err(Error::from_win32());
        }
        let entry = Rc::new(Entry {
            identity,
            fired: Cell::new(false),
            notify,
        });
        ENTRIES.with(|entries| {
            entries
                .borrow_mut()
                .push((hook.0 as isize, Rc::downgrade(&entry)))
        });
        Ok(Self {
            hook,
            _entry: entry,
        })
    }
}
impl Drop for Watcher {
    fn drop(&mut self) {
        // Rc 禁止跨线程移动；先移除注册，再注销，迟到回调不再找到通知目标。
        ENTRIES.with(|entries| {
            entries
                .borrow_mut()
                .retain(|(hook, _)| *hook != self.hook.0 as isize)
        });
        unsafe {
            if !UnhookWinEvent(self.hook).as_bool() {
                eprintln!("native-live-destroy-unhook: failed");
            }
        }
    }
}
unsafe extern "system" fn on_event(
    hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    child: i32,
    thread: u32,
    _: u32,
) {
    if event != EVENT_OBJECT_DESTROY || object != OBJID_WINDOW.0 || child != 0 {
        return;
    }
    let entry = ENTRIES.with(|entries| {
        entries
            .borrow()
            .iter()
            .find(|(id, _)| *id == hook.0 as isize)
            .and_then(|(_, entry)| entry.upgrade())
    });
    if let Some(entry) = entry {
        if entry.identity.window == hwnd.0 as usize
            && entry.identity.thread == thread
            && !entry.fired.replace(true)
        {
            // 不持有注册表借用，通知仅关闭共享帧源以唤醒 worker，不能在回调中 join。
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (entry.notify)())).is_err()
            {
                eprintln!("native-live-destroy-notify: callback panicked");
            }
        }
    }
}

#[cfg(test)]
#[path = "live_window_watch_tests.rs"]
mod tests;
