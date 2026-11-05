//! Live 最新帧交付；单生产者、有界消费者、共享资源而非每消费者复制/排队。
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
#[path = "live_frame_wake.rs"]
mod wake;
pub const MAX_CONSUMERS: usize = 16;
pub struct Frame<T> {
    pub sequence: u64,
    pub resource: T,
}
pub struct Delivery<T> {
    pub frame: Arc<Frame<T>>,
    pub skipped: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    NoConsumers,
    ConsumerLimit,
    SequenceExhausted,
    OwnerConflict,
}
struct State<T> {
    latest: Option<Arc<Frame<T>>>,
    sequence: u64,
    consumers: usize,
    closed: bool,
    idle: Option<Arc<dyn Fn() + Send + Sync>>,
    wakes: Vec<Weak<wake::Wake>>,
}
struct Shared<T> {
    state: Mutex<State<T>>,
    changed: Condvar,
}
impl<T> Shared<T> {
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        // 锁内只有内部元数据操作，不调用外部代码；析构资源必须移到锁外。
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}
/// 不可克隆的源 owner；析构关闭所有消费者，但不强制回收已借出的帧。
pub struct Source<T> {
    shared: Arc<Shared<T>>,
}
pub struct Consumer<T> {
    shared: Arc<Shared<T>>,
    cursor: u64,
    wake: Option<Arc<wake::Wake>>,
}
impl<T> Default for Source<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Source<T> {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    latest: None,
                    sequence: 0,
                    consumers: 0,
                    closed: false,
                    idle: None,
                    wakes: Vec::new(),
                }),
                changed: Condvar::new(),
            }),
        }
    }
    pub fn subscribe(&self) -> Result<Consumer<T>, Error> {
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        if state.consumers == MAX_CONSUMERS {
            return Err(Error::ConsumerLimit);
        }
        state.consumers += 1;
        // 新消费者可立即取当前帧，但不将订阅前的帧记为自己丢帧。
        let cursor = if state.latest.is_some() {
            state.sequence - 1
        } else {
            state.sequence
        };
        Ok(Consumer {
            shared: self.shared.clone(),
            cursor,
            wake: None,
        })
    }
    pub fn has_consumers(&self) -> bool {
        let state = self.shared.lock();
        !state.closed && state.consumers != 0
    }
    /// 捕获 owner 注册一次最后消费者退出通知；回调只负责唤醒，不在状态锁内执行。
    pub(crate) fn on_idle(&self, wake: Arc<dyn Fn() + Send + Sync>) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        if state.idle.is_some() {
            return Err(Error::OwnerConflict);
        }
        let idle = state.consumers == 0;
        state.idle = Some(wake.clone());
        drop(state);
        if idle {
            wake();
        }
        Ok(())
    }
    pub fn publish(&self, resource: T) -> Result<u64, Error> {
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        if state.consumers == 0 {
            return Err(Error::NoConsumers);
        }
        let sequence = state
            .sequence
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        let previous = state.latest.replace(Arc::new(Frame { sequence, resource }));
        state.sequence = sequence;
        let wakes = observers(&state);
        drop(state);
        self.shared.changed.notify_all();
        for wake in wakes.into_iter().flatten() {
            wake.notify();
        }
        drop(previous);
        Ok(sequence)
    }
    pub fn close(&self) {
        let mut state = self.shared.lock();
        if state.closed {
            return;
        }
        state.closed = true;
        let previous = state.latest.take();
        let idle = state.idle.take();
        let wakes = observers(&state);
        drop(state);
        self.shared.changed.notify_all();
        for wake in wakes.into_iter().flatten() {
            wake.notify();
        }
        if let Some(wake) = idle {
            wake();
        }
        drop(previous);
    }
}
impl<T> Drop for Source<T> {
    fn drop(&mut self) {
        self.close();
    }
}
impl<T> Consumer<T> {
    pub fn notification_failures(&self) -> u64 {
        self.wake.as_ref().map_or(0, |wake| wake.failures())
    }
    /// 非阻塞通知回调，返回是否成功投递；一次绑定，不在源锁内调用。不携带像素或指针。
    /// UI 必须验证自身连接代次：取消不能撤销已经进入系统消息队列的通知。
    pub fn set_waker(
        &mut self,
        post: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<(), Error> {
        if self.wake.is_some() {
            return Err(Error::OwnerConflict);
        }
        let wake = Arc::new(wake::Wake::new(post));
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        state.wakes.retain(|entry| entry.strong_count() != 0);
        if state.wakes.len() == MAX_CONSUMERS {
            return Err(Error::ConsumerLimit);
        }
        state.wakes.push(Arc::downgrade(&wake));
        let ready = state
            .latest
            .as_ref()
            .is_some_and(|frame| frame.sequence > self.cursor);
        self.wake = Some(wake.clone());
        drop(state);
        if ready {
            wake.notify();
        }
        Ok(())
    }
    fn take(state: &State<T>, cursor: &mut u64) -> Result<Option<Delivery<T>>, Error> {
        if state.closed {
            return Err(Error::Closed);
        }
        let Some(frame) = state
            .latest
            .as_ref()
            .filter(|frame| frame.sequence > *cursor)
        else {
            return Ok(None);
        };
        let skipped = frame.sequence - *cursor - 1;
        *cursor = frame.sequence;
        Ok(Some(Delivery {
            frame: frame.clone(),
            skipped,
        }))
    }
    /// UI 在现有事件通知后尝试读取；不启动任何轮询线程。
    pub fn try_next(&mut self) -> Result<Option<Delivery<T>>, Error> {
        let state = self.shared.lock();
        if let Some(wake) = &self.wake {
            wake.acknowledge();
        }
        Self::take(&state, &mut self.cursor)
    }
    /// 供后台消费者阻塞等待；关闭源会唤醒，不设置周期性超时。
    pub fn wait_next(&mut self) -> Result<Delivery<T>, Error> {
        let mut state = self.shared.lock();
        loop {
            if let Some(wake) = &self.wake {
                wake.acknowledge();
            }
            if let Some(frame) = Self::take(&state, &mut self.cursor)? {
                return Ok(frame);
            }
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}
impl<T> Drop for Consumer<T> {
    fn drop(&mut self) {
        let retired = self.wake.take();
        if let Some(wake) = &retired {
            wake.disable();
        }
        let mut state = self.shared.lock();
        if let Some(wake) = &retired {
            let identity = Arc::downgrade(wake);
            state.wakes.retain(|entry| !Weak::ptr_eq(entry, &identity));
        }
        state.consumers -= 1;
        let previous = if state.consumers == 0 {
            state.latest.take()
        } else {
            None
        };
        let idle = (state.consumers == 0).then(|| state.idle.clone()).flatten();
        drop(state);
        drop(retired);
        if let Some(wake) = idle {
            wake();
        }
        drop(previous);
    }
}
fn observers<T>(state: &State<T>) -> [Option<Arc<wake::Wake>>; MAX_CONSUMERS] {
    // 固定栈数组，热路径不分配通知队列；弱引用不延长消费者生命周期。
    std::array::from_fn(|index| state.wakes.get(index).and_then(Weak::upgrade))
}
#[cfg(test)]
#[path = "live_frames_tests.rs"]
mod tests;
