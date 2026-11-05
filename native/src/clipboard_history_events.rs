//! 历史菜单的事件订阅生命周期；回调只合并失效信号，不读取内容、不访问窗口。
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
        // 包含部分注册失败的路径；一项注销失败也继续尝试其他订阅。
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

/// 在拥有消息泵的 UI STA 创建和销毁；不能跨线程移动。
/// 读取刷新标记后启动 Task，读取期间的新事件保留给下一次刷新。
pub struct Events {
    dirty: Arc<AtomicBool>,
    // 按字段顺序先注销，最后退出 apartment；迟到回调仅捕获 Weak。
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

    /// 非阻塞合并通知；初次返回 true，事件洪泛不会积累队列或历史内容。
    pub fn take_changed(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }
}

#[cfg(test)]
#[path = "clipboard_history_events_tests.rs"]
mod tests;
