//! 独占控制游标与媒体许可；只在完整消费后提交，暂态GET损失交原runtime有限恢复，其余失败关闭。
use crate::{
    live_relay_http::{Control, Error as HttpError, Prepared, RequesterCursor},
    live_relay_viewer_identity::{ControlBatch, ControlCursor, ControlEvent, Snapshot},
    live_relay_viewer_session::{Permit, ViewerSession},
};
use std::sync::Arc;
#[path = "live_relay_control_state.rs"]
mod state;
pub use state::Visibility;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Protocol,
    Consumer,
    Media,
    Http(HttpError),
    Business(crate::live_relay_viewer_business::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub closed: bool,
    pub controller_owned: bool,
}
pub struct Owner {
    business: crate::live_relay_viewer_business::Business,
    snapshot: Snapshot,
    cursor: ControlCursor,
    media: Arc<ViewerSession>,
    permit: Option<Permit>,
    device: String,
    revision: u64,
    baseline: RequesterCursor,
    controller: Option<String>,
    visibility: Option<Visibility>,
    closed: bool,
}
impl Owner {
    pub fn new(prepared: Prepared, device: String) -> Result<Self, Error> {
        prepared
            .snapshot
            .renewal(prepared.snapshot.identity(), &device)
            .map_err(|_| Error::Protocol)?;
        let media = Arc::new(
            ViewerSession::new(prepared.snapshot.identity().clone(), device.clone())
                .map_err(|_| Error::Media)?,
        );
        Ok(Self {
            business: prepared.business,
            revision: prepared.snapshot.revision(),
            snapshot: prepared.snapshot,
            cursor: prepared.cursor,
            media,
            permit: None,
            device,
            baseline: RequesterCursor {
                control: prepared.control_sequence,
                input: prepared.input_sequence,
            },
            controller: None,
            visibility: None,
            closed: false,
        })
    }
    pub fn media(&self) -> Arc<ViewerSession> {
        self.media.clone()
    }
    pub(crate) fn identity(&self) -> &crate::live_relay_viewer_identity::Identity {
        self.snapshot.identity()
    }
    pub fn business(&self) -> &crate::live_relay_viewer_business::Business {
        &self.business
    }
    pub fn after(&self) -> u64 {
        self.cursor.after()
    }
    /// 只更新已对账注册，不推进可靠cursor或导入可能超前的observation/audit。
    pub(crate) fn registrations(
        &mut self,
        registrations: Vec<crate::live_relay_trigger_registration::Registration>,
    ) -> Result<(), Error> {
        let permit = self.permit()?;
        let mut business = self.business.clone();
        business.replace_registrations(registrations);
        self.media
            .publish_business(&permit, Arc::new(business.clone()))
            .map_err(|_| Error::Media)?;
        self.business = business;
        Ok(())
    }
    pub fn baseline(&self) -> RequesterCursor {
        self.baseline
    }
    pub fn visibility(&self) -> Option<Visibility> {
        self.visibility
    }
    pub fn permit(&self) -> Result<Permit, Error> {
        let permit = self.permit.as_ref().ok_or(Error::Closed)?;
        self.media.validate(permit).map_err(|_| Error::Media)?;
        Ok(permit.clone())
    }
    /// 仅后台调用。业务 consumer 收到全部事件（包含 observation/trigger），不得隐式丢弃。
    /// consumer 必须完成自己的批次校验/提交，失败不会推进控制游标，并使本观看关闭。
    pub fn poll(
        &mut self,
        control: &Control<'_>,
        consume: impl FnOnce(&[ControlEvent]) -> Result<(), ()>,
    ) -> Result<Applied, Error> {
        self.poll_wait(control, consume, 750)
    }
    pub(crate) fn poll_wait(
        &mut self,
        control: &Control<'_>,
        consume: impl FnOnce(&[ControlEvent]) -> Result<(), ()>,
        wait: u16,
    ) -> Result<Applied, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let batch =
            match control.events_wait(self.snapshot.identity().session(), &self.cursor, wait) {
                Ok(batch) => batch,
                // 没有批次被解析/消费/提交；保原cursor、Permit及媒体，不能重新bootstrap跳过历史。
                Err(error) if error.retryable_read() => return Err(Error::Http(error)),
                Err(error) => {
                    // 服务端可能关闭后直接返回 404；先停旧媒体，再单独验证成员关闭事实。
                    self.close();
                    if error == HttpError::Status(404)
                        && control.terminal(self.snapshot.identity()) == Ok(true)
                    {
                        return Ok(Applied {
                            closed: true,
                            controller_owned: false,
                        });
                    }
                    return Err(Error::Http(error));
                }
            };
        self.apply(batch, consume)
    }
    pub fn apply(
        &mut self,
        batch: ControlBatch,
        consume: impl FnOnce(&[ControlEvent]) -> Result<(), ()>,
    ) -> Result<Applied, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let result = self.apply_inner(&batch, consume);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn apply_inner(
        &mut self,
        batch: &ControlBatch,
        consume: impl FnOnce(&[ControlEvent]) -> Result<(), ()>,
    ) -> Result<Applied, Error> {
        self.cursor.validate(batch).map_err(|_| Error::Protocol)?;
        if let Some(permit) = &self.permit {
            self.media.validate(permit).map_err(|_| Error::Media)?;
        }
        let reduced = state::reduce(
            batch.events(),
            &self.device,
            self.revision,
            self.permit.is_none(),
            self.controller.clone(),
            self.visibility,
        )?;
        // 成员缺失或 closed 是不可逆终态；同批次较晚的重新加入不得抹去撤权。
        if reduced.closed {
            self.close();
        }
        let mut business = self.business.clone();
        business.apply(batch.events()).map_err(Error::Business)?;
        consume(batch.events()).map_err(|_| Error::Consumer)?;
        if !reduced.closed {
            if self.permit.is_none() {
                self.media
                    .activate(&self.snapshot)
                    .map_err(|_| Error::Media)?;
                self.permit = Some(
                    self.media
                        .admit(self.snapshot.identity().epoch())
                        .map_err(|_| Error::Media)?,
                );
            }
            self.media
                .advance_control(self.permit.as_ref().ok_or(Error::Media)?, reduced.revision)
                .map_err(|_| Error::Media)?;
            if batch.next() != self.cursor.after() {
                self.media
                    .publish_business(
                        self.permit.as_ref().ok_or(Error::Media)?,
                        Arc::new(business.clone()),
                    )
                    .map_err(|_| Error::Media)?;
            }
        }
        self.cursor.commit(batch).map_err(|_| Error::Protocol)?;
        self.business = business;
        self.revision = reduced.revision;
        self.visibility = reduced.visibility;
        self.controller = if reduced.closed {
            None
        } else {
            reduced.controller
        };
        Ok(Applied {
            closed: self.closed,
            controller_owned: self.controller.as_deref() == Some(&self.device),
        })
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.controller = None;
        self.permit = None;
        self.media.close();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
#[path = "live_relay_control_owner_tests.rs"]
mod tests;
