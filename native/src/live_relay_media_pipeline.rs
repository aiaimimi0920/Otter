//! codec 线程内的媒体流水线：固定许可→解码→同锁发布；不持有 socket 或 UI。
use crate::{
    desktop::live_relay_image,
    live_relay_receive::Frame,
    live_relay_video_receiver::{self as video, Outcome, Receiver},
    live_relay_viewer_frames::Commit,
    live_relay_viewer_session::{self as session, Permit, ViewerSession},
};
use std::{marker::PhantomData, rc::Rc, sync::Arc};
use windows::{
    core::HRESULT,
    Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Session(session::Error),
    Receiver(video::Error),
    Media(HRESULT),
    Retired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Published(Commit),
    Pending,
    RequestKeyframe { epoch: u64 },
    Fallback { epoch: u64, reason: video::Failure },
}
struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    fn new() -> Result<Self, Error> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| Error::Media(e.code()))?;
        }
        Ok(Self(PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
pub struct Pipeline {
    // 字段析构顺序：decoder 必须先于外层 MTA apartment 释放。
    receiver: Receiver,
    _apartment: Apartment,
    session: Arc<ViewerSession>,
    permit: Permit,
    retired: bool,
}
impl Pipeline {
    /// 在将执行所有 receive/check 的同一后台线程创建；Rc marker 禁止跨线程搬运。
    pub fn new(session: Arc<ViewerSession>, permit: Permit) -> Result<Self, Error> {
        session.validate(&permit).map_err(Error::Session)?;
        let apartment = Apartment::new()?;
        Ok(Self {
            receiver: Receiver::default(),
            _apartment: apartment,
            session,
            permit,
            retired: false,
        })
    }
    /// 网络循环在读取边界/取消唤醒时调用；无需等下一张图片才释放 decoder。
    pub fn check(&mut self) -> Result<(), Error> {
        if self.retired {
            return Err(Error::Retired);
        }
        if let Err(error) = self.session.validate(&self.permit) {
            self.retire();
            return Err(Error::Session(error));
        }
        Ok(())
    }
    pub fn retire(&mut self) {
        self.receiver.close();
        self.retired = true;
    }
    pub fn receive(&mut self, frame: Frame) -> Result<Progress, Error> {
        self.check()?;
        let result = self.receive_inner(frame);
        if result.is_err() {
            self.retire();
        }
        result
    }
    fn receive_inner(&mut self, frame: Frame) -> Result<Progress, Error> {
        let outcome = self
            .receiver
            .receive(frame, self.permit.epoch(), self.permit.stop())
            .map_err(Error::Receiver)?;
        let (pixels, warmup) = match outcome {
            Outcome::Image(frame) => (
                live_relay_image::decode_pixels(&frame).map_err(|e| Error::Media(e.code()))?,
                false,
            ),
            Outcome::Decoded(frame) => (frame, true),
            Outcome::Pending => {
                self.check()?;
                return Ok(Progress::Pending);
            }
            Outcome::RequestKeyframe { epoch } => {
                self.check()?;
                return Ok(Progress::RequestKeyframe { epoch });
            }
            Outcome::Fallback { epoch, reason } => {
                self.check()?;
                return Ok(Progress::Fallback { epoch, reason });
            }
        };
        // 解码期间撤权/关闭由 commit 在同锁下拒绝，失败结果不会进入 viewer 缓存。
        self.session
            .commit(&self.permit, pixels, warmup)
            .map(Progress::Published)
            .map_err(Error::Session)
    }
}
#[cfg(test)]
#[path = "live_relay_media_pipeline_tests.rs"]
mod tests;
