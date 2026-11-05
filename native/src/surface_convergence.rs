//! 单动作流收敛：成功 ACK 后单独刷新快照，完整检查批次才允许交付。
use crate::{
    surface_action_ack::{Ack, Status},
    surface_click_event::ClickEvent,
    surface_stream::Batch,
};
use serde::Deserialize;
use serde_json::value::RawValue;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Identity,
    Protocol,
    Cursor,
    Reset,
    Generation,
    Inactive,
    Action,
    Unknown,
    State,
    Ack(crate::surface_action_ack::Error),
    Scene(crate::surface_scene::Error),
    Result(crate::surface_result_commit::Error),
    Confirmation(crate::surface_confirmation::Error),
    Resource(crate::surface_resource_snapshot::Error),
}
pub struct Snapshot {
    pub scene: crate::surface_scene::Scene,
    pub revision: u64,
    pub commit: Option<crate::surface_result_commit::Commit>,
    pub(crate) resources: crate::surface_resource_snapshot::ResourceSnapshot,
}
impl Snapshot {
    /// 与本次完整收敛快照同行的资源身份；仍不是图片读取或长期显示许可。
    pub fn resources(&self) -> &crate::surface_resource_snapshot::ResourceSnapshot {
        &self.resources
    }
}
pub struct Tracker<'a> {
    event: &'a ClickEvent,
    ack: Ack,
    cursor: u64,
    success: bool,
    refresh: bool,
    finished: bool,
    failure: Option<Error>,
    snapshot: Option<Snapshot>,
    commit: Option<crate::surface_result_commit::Commit>,
    confirmation: Option<crate::surface_confirmation::Request>,
}
#[derive(Deserialize)]
struct Message<'a> {
    method: &'a str,
    #[serde(borrow)]
    params: Option<&'a RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header<'a> {
    protocol_version: Option<&'a str>,
    instance_id: Option<&'a str>,
    attachment_id: Option<&'a str>,
    event_id: Option<&'a str>,
    generation: Option<u64>,
    revision: Option<u64>,
}
#[derive(Deserialize)]
struct LifecycleState<'a> {
    state: Option<&'a str>,
}
#[derive(Deserialize)]
struct Delivery<'a> {
    generation: Option<u64>,
    #[serde(borrow)]
    snapshot: &'a RawValue,
}
#[derive(Deserialize)]
struct Lifecycle<'a> {
    #[serde(borrow)]
    event: &'a RawValue,
}
impl<'a> Tracker<'a> {
    /// 借用事件、复制有界 ACK；构造失败不消费调用方执行证据。
    pub fn new(event: &'a ClickEvent, ack: &Ack) -> Result<Self, Error> {
        if !ack.matches_event(event) {
            return Err(Error::Identity);
        }
        let mut tracker = Self {
            event,
            ack: ack.clone(),
            cursor: 0,
            success: ack.succeeded(),
            refresh: ack.succeeded(),
            finished: false,
            failure: None,
            snapshot: None,
            commit: None,
            confirmation: None,
        };
        tracker.check_status()?;
        Ok(tracker)
    }
    pub fn after(&self) -> u64 {
        if self.refresh {
            0
        } else {
            self.cursor
        }
    }
    pub fn ack(&self) -> &Ack {
        &self.ack
    }
    pub fn failure(&self) -> Option<Error> {
        self.failure
    }
    pub fn take_snapshot(&mut self) -> Option<Snapshot> {
        self.snapshot.take()
    }
    pub fn take_confirmation(&mut self) -> Option<crate::surface_confirmation::Request> {
        self.confirmation.take()
    }
    pub fn apply(&mut self, batch: &Batch) -> Result<(), Error> {
        if self.finished || self.failure.is_some() {
            return Err(Error::State);
        }
        let result = self.apply_batch(batch);
        if let Err(error) = result {
            self.failure = Some(error);
            self.snapshot = None;
            self.commit = None;
            self.confirmation = None;
        }
        result
    }
    fn check_status(&mut self) -> Result<(), Error> {
        if !self.ack.accepted()
            || matches!(
                self.ack.status(),
                Status::Failed | Status::Cancelled | Status::Interrupted
            )
        {
            return Err(Error::Action);
        }
        if self.ack.status() == Status::Unknown {
            return Err(Error::Unknown);
        }
        self.success |= self.ack.succeeded();
        Ok(())
    }
    fn same_attachment(&self, header: &Header<'_>) -> bool {
        header.instance_id == Some(self.event.binding().instance())
            && header.attachment_id == Some(self.event.binding().attachment())
    }
    fn apply_batch(&mut self, batch: &Batch) -> Result<(), Error> {
        if batch.after() != self.after() {
            return Err(Error::Cursor);
        }
        if batch.reset() && batch.after() != 0 {
            return Err(Error::Reset);
        }
        let mut latest: Option<(u64, &RawValue)> = None;
        for raw in batch.messages() {
            let message: Message<'_> =
                serde_json::from_str(raw.get()).map_err(|_| Error::Protocol)?;
            if !matches!(
                message.method,
                "loom.surface.action.ack"
                    | "loom.surface.result"
                    | "loom.surface.confirmation.request"
                    | "loom.surface.snapshot"
                    | "loom.surface.generation"
                    | "loom.surface.dispose"
                    | "loom.surface.lifecycle"
            ) {
                continue;
            }
            let params = message.params.ok_or(Error::Protocol)?;
            match message.method {
                "loom.surface.confirmation.request" => {
                    if let Some(request) = crate::surface_confirmation::Request::parse_matching(
                        params, self.event, &self.ack,
                    )
                    .map_err(Error::Confirmation)?
                    {
                        // 同一动作只允许一个确认身份，不能用迟到请求替换已展示意图。
                        if self.confirmation.as_ref().is_some_and(|old| {
                            old.id() != request.id() || !old.same_prompt(&request)
                        }) {
                            return Err(Error::Confirmation(
                                crate::surface_confirmation::Error::Identity,
                            ));
                        }
                        self.confirmation = Some(request);
                    }
                }
                "loom.surface.result" => {
                    #[derive(Deserialize)]
                    struct ResultDelivery<'a> {
                        #[serde(borrow)]
                        commit: &'a RawValue,
                    }
                    let delivery: ResultDelivery<'_> =
                        serde_json::from_str(params.get()).map_err(|_| Error::Protocol)?;
                    if let Some(commit) = crate::surface_result_commit::Commit::parse_matching(
                        delivery.commit,
                        self.event,
                        &self.ack,
                    )
                    .map_err(Error::Result)?
                    {
                        if self
                            .commit
                            .as_ref()
                            .is_none_or(|old| commit.revision() > old.revision())
                        {
                            self.commit = Some(commit);
                        }
                    }
                }
                "loom.surface.action.ack" => {
                    let header: Header<'_> =
                        serde_json::from_str(params.get()).map_err(|_| Error::Protocol)?;
                    if header.instance_id != Some(self.event.binding().instance())
                        || header.event_id != Some(self.event.id())
                    {
                        continue;
                    }
                    let previous = self.ack.clone();
                    let was_success = self.success;
                    self.ack
                        .update(params.get().as_bytes(), self.event)
                        .map_err(Error::Ack)?;
                    self.check_status()?;
                    // after=0 刷新可能重放早期 running；不降级已知成功，但上面的失败仍终止。
                    if was_success && !self.ack.succeeded() {
                        self.ack = previous;
                    }
                }
                "loom.surface.snapshot" => {
                    let delivery: Delivery<'_> =
                        serde_json::from_str(params.get()).map_err(|_| Error::Protocol)?;
                    let header: Header<'_> = serde_json::from_str(delivery.snapshot.get())
                        .map_err(|_| Error::Protocol)?;
                    if !self.same_attachment(&header) {
                        continue;
                    }
                    let generation = delivery.generation.ok_or(Error::Protocol)?;
                    if generation < self.event.binding().generation() {
                        continue;
                    }
                    if generation > self.event.binding().generation() {
                        return Err(Error::Generation);
                    }
                    if header.protocol_version != Some("loom.surface.v1") {
                        return Err(Error::Protocol);
                    }
                    let revision = header.revision.ok_or(Error::Protocol)?;
                    if self.refresh
                        && revision >= self.event.revision()
                        && latest.as_ref().is_none_or(|(old, _)| revision >= *old)
                    {
                        latest = Some((revision, delivery.snapshot));
                    }
                }
                "loom.surface.lifecycle" => {
                    let delivery: Lifecycle<'_> =
                        serde_json::from_str(params.get()).map_err(|_| Error::Protocol)?;
                    let header: Header<'_> =
                        serde_json::from_str(delivery.event.get()).map_err(|_| Error::Protocol)?;
                    // 快照 state 是任意业务数据；只有生命周期事件的 state 是严格字符串。
                    let lifecycle: LifecycleState<'_> =
                        serde_json::from_str(delivery.event.get()).map_err(|_| Error::Protocol)?;
                    if self.same_attachment(&header) {
                        if header.protocol_version != Some("loom.surface.v1") {
                            return Err(Error::Protocol);
                        }
                        let revision = header.revision.ok_or(Error::Protocol)?;
                        if revision >= self.event.binding().lifecycle_revision()
                            && lifecycle.state != Some("active")
                        {
                            return Err(Error::Inactive);
                        }
                    }
                }
                _ => {
                    let header: Header<'_> =
                        serde_json::from_str(params.get()).map_err(|_| Error::Protocol)?;
                    if !self.same_attachment(&header) {
                        continue;
                    }
                    if message.method == "loom.surface.dispose" {
                        return Err(Error::Inactive);
                    }
                    if header.generation.ok_or(Error::Protocol)? > self.event.binding().generation()
                    {
                        return Err(Error::Generation);
                    }
                }
            }
        }
        // 不能在看到第一条成功/快照时返回，后续失败或代际改变必须胜过旧快照。
        if self.refresh && self.success {
            if let Some((revision, raw)) = latest {
                use crate::surface_host_capabilities::Runtime;
                let expected = if self.event.is_javascript() {
                    Runtime::Javascript
                } else {
                    Runtime::Declarative
                };
                // JS scene 仅是动作权威；窗口必须异步 update 同 child 后才能报告呈现成功。
                let scene = crate::live_relay_http::scene_for_runtime(raw.get(), expected)
                    .map_err(Error::Scene)?;
                self.snapshot = Some(Snapshot {
                    scene,
                    revision,
                    commit: self.commit.take(),
                    resources: crate::surface_resource_snapshot::ResourceSnapshot::new(
                        raw,
                        self.event.binding(),
                        revision,
                    )
                    .map_err(Error::Resource)?,
                });
                self.finished = true;
            }
        }
        self.cursor = batch.next();
        self.refresh = self.success;
        Ok(())
    }
}
