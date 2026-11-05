//! 会话媒体发布 gate；state→frames 固定锁序，关闭/授权失效与发布不可交错。
use crate::{
    live_relay_frame::Metadata,
    live_relay_pixels::PixelFrame,
    live_relay_viewer_frames::{Commit, Cursor, ViewerPixels},
    live_relay_viewer_identity::{self as authority, Identity, Snapshot},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
#[path = "live_relay_frame_notice.rs"]
mod notice;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Suspended,
    Epoch,
    StalePermit,
    Authority,
    Cache,
    Poisoned,
    Notice,
}
#[derive(Clone)]
pub struct Permit {
    owner: Arc<()>,
    generation: u64,
    epoch: u64,
    stop: Arc<AtomicBool>,
}
impl Permit {
    /// 只作 worker/codec 取消信号；发布授权仍必须通过同锁 commit 复核。
    pub fn stop(&self) -> &AtomicBool {
        &self.stop
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
struct State {
    business: Option<Arc<crate::live_relay_viewer_business::Business>>,
    active: bool,
    closed: bool,
    generation: u64,
    frames: ViewerPixels,
    revision: u64,
    stop: Arc<AtomicBool>,
}
pub struct ViewerSession {
    owner: Arc<()>,
    identity: Identity,
    device: String,
    state: Mutex<State>,
    changed: notice::Notice,
}
impl ViewerSession {
    pub(super) fn identity(&self) -> &Identity {
        &self.identity
    }
    pub(crate) fn notify_input(&self) {
        self.changed.signal();
    }
    pub(super) fn matches_media_target(&self, session: &str, device: &str) -> bool {
        self.identity.session() == session && self.device == device
    }
    /// 新 owner 总是暂停；identity/设备来自已认证加入流程，不从媒体包构造。
    pub fn new(identity: Identity, device: String) -> Result<Self, Error> {
        if !authority::identifier(&device) {
            return Err(Error::Authority);
        }
        let frames = ViewerPixels::new(identity.epoch()).map_err(|_| Error::Epoch)?;
        Ok(Self {
            changed: notice::Notice::new().map_err(|_| Error::Notice)?,
            owner: Arc::new(()),
            identity,
            device,
            state: Mutex::new(State {
                business: None,
                active: false,
                closed: false,
                generation: 0,
                revision: 0,
                stop: Arc::new(AtomicBool::new(true)),
                frames,
            }),
        })
    }
    /// 仅由可信控制 owner 在认证、bootstrap 重放及业务校验完成后调用。
    /// 本方法验证成员/来源合同，不执行认证；每次激活都撤销旧工作许可并丢弃旧缓存。
    pub fn activate(&self, snapshot: &Snapshot) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        // 迟到快照不能覆盖较新的控制状态，也不能无故撤销新代次的工作。
        if snapshot.revision() < state.revision {
            return Err(Error::Authority);
        }
        self.invalidate(&mut state)?;
        if snapshot.renewal(&self.identity, &self.device).is_err() {
            state.closed = true;
            state.stop.store(true, Ordering::Release);
            state.frames.close();
            return Err(Error::Authority);
        }
        state.revision = snapshot.revision();
        state.active = true;
        Ok(())
    }
    fn invalidate(&self, state: &mut State) -> Result<(), Error> {
        state.business = None;
        state.active = false;
        state.stop.store(true, Ordering::Release);
        state.frames.close();
        self.changed.signal();
        let Some(next) = state.generation.checked_add(1) else {
            state.closed = true;
            return Err(Error::Closed);
        };
        state.generation = next;
        state.stop = Arc::new(AtomicBool::new(false));
        state.frames = ViewerPixels::new(self.identity.epoch()).map_err(|_| Error::Epoch)?;
        Ok(())
    }
    pub fn suspend(&self) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        self.invalidate(&mut state)
    }
    /// 旧 worker 的回收请求不得撤销后来激活的新代次。
    pub fn suspend_if_current(&self, permit: &Permit) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        self.invalidate(&mut state)
    }
    pub fn admit(&self, epoch: u64) -> Result<Permit, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Self::ready(&state)?;
        if epoch != self.identity.epoch() {
            return Err(Error::Epoch);
        }
        Ok(Permit {
            owner: self.owner.clone(),
            generation: state.generation,
            epoch,
            stop: state.stop.clone(),
        })
    }
    pub fn commit(
        &self,
        permit: &Permit,
        frame: PixelFrame,
        allow_warmup: bool,
    ) -> Result<Commit, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        if frame.metadata().epoch != permit.epoch {
            return Err(Error::Epoch);
        }
        // 保持 gate 锁直到缓存提交完成；不会在此锁内解码、做网络 I/O 或等待 worker。
        let result = state
            .frames
            .commit(frame, allow_warmup)
            .map_err(|_| Error::Cache)?;
        if result == Commit::Accepted {
            self.changed.signal();
        }
        Ok(result)
    }
    /// worker 开始工作/发送控制前复核固定许可；不能靠重新 admit 绕过代次失效。
    pub fn validate(&self, permit: &Permit) -> Result<(), Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)
    }
    /// 恢复只读已真正提交的像素水位；UI取走缓存不丢水位，不从wire/排队冒充解码成功。
    pub(super) fn committed(&self, permit: &Permit) -> Result<Cursor, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        state
            .frames
            .status()
            .map(|status| status.cursor)
            .map_err(|_| Error::Cache)
    }
    /// 同一许可的控制进度只更新 revision，不重建帧缓存；旧控制 owner 不能复活新代次。
    pub(super) fn advance_control(&self, permit: &Permit, revision: u64) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        if revision < state.revision {
            return Err(Error::Authority);
        }
        state.revision = revision;
        Ok(())
    }
    fn verify(&self, state: &State, permit: &Permit) -> Result<(), Error> {
        Self::ready(state)?;
        if !Arc::ptr_eq(&self.owner, &permit.owner) || state.generation != permit.generation {
            return Err(Error::StalePermit);
        }
        Ok(())
    }
    pub fn latest_after(&self, cursor: Cursor) -> Result<Option<Metadata>, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Self::ready(&state)?;
        state.frames.latest_after(cursor).map_err(|_| Error::Cache)
    }
    pub fn take(&self, cursor: Cursor) -> Result<Option<PixelFrame>, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Self::ready(&state)?;
        state.frames.take(cursor).map_err(|_| Error::Cache)
    }
    /// 单个 UI consumer 等待此合并事件并调用 poll_frame；句柄不可比 Session 活得更久。
    #[cfg(windows)]
    pub fn change_handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        self.changed.handle()
    }
    /// 同锁清除通知并取最新帧；暂停/关闭也会唤醒，调用者必须撤下旧显示资源。
    pub fn poll_frame(&self, after: Cursor) -> Result<Option<PixelFrame>, Error> {
        self.poll_update(after).map(|update| update.frame)
    }
    /// 同一次锁内获取帧与业务快照并确认通知，避免业务更新与 reset 交错丢失唤醒。
    pub fn poll_update(&self, after: Cursor) -> Result<Update, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.changed.reset();
        Self::ready(&state)?;
        let frame = match state.frames.latest_after(after).map_err(|_| Error::Cache)? {
            Some(metadata) => state
                .frames
                .take(metadata.into())
                .map_err(|_| Error::Cache)?,
            None => None,
        };
        Ok(Update {
            frame,
            business: state.business.clone(),
        })
    }
    pub(super) fn publish_business(
        &self,
        permit: &Permit,
        business: Arc<crate::live_relay_viewer_business::Business>,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        state.business = Some(business);
        self.changed.signal();
        Ok(())
    }
    fn ready(state: &State) -> Result<(), Error> {
        if state.closed {
            Err(Error::Closed)
        } else if !state.active {
            Err(Error::Suspended)
        } else {
            Ok(())
        }
    }
    pub fn close(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.active = false;
        state.closed = true;
        state.business = None;
        state.stop.store(true, Ordering::Release);
        state.frames.close();
        self.changed.signal();
    }
    /// 只处理当前媒体连接的权威撤销；旧连接不得关闭后来激活的新代次。
    pub fn close_if_current(&self, permit: &Permit) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        state.active = false;
        state.closed = true;
        state.business = None;
        state.stop.store(true, Ordering::Release);
        state.frames.close();
        self.changed.signal();
        Ok(())
    }
    /// 迟到 open 只表示远端快照开放，不激活本地 gate。HTTP 错误不可送作关闭事实。
    pub fn observe_terminal(&self, bytes: &[u8]) -> Result<bool, Error> {
        let closed = authority::terminal_snapshot(bytes, &self.identity, &self.device)
            .map_err(|_| Error::Authority)?;
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if closed {
            state.active = false;
            state.closed = true;
            state.business = None;
            state.stop.store(true, Ordering::Release);
            state.frames.close();
            self.changed.signal();
        }
        Ok(state.closed)
    }
}
pub struct Update {
    pub frame: Option<PixelFrame>,
    pub business: Option<Arc<crate::live_relay_viewer_business::Business>>,
}
#[cfg(test)]
#[path = "live_relay_viewer_session_tests.rs"]
mod tests;
