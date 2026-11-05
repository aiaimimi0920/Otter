//! 图准备结果到后台执行 owner 的注册事务；先校验资源和编码，最后才替换请求代际。
use super::{loom_execution_job::Execution, loom_input_job::PreparedBatch};
use crate::{
    editor::Editor,
    graph_image::Prepared,
    graph_requests::Requests,
    loom_art_wire::{self, OutputTransport},
    loom_bridge_identity::Identity,
    workspace_id::UnitId,
};
use std::time::Duration;

pub struct Plan<'a, 'graph> {
    pub identity: Identity,
    pub request_id: UnitId,
    pub art_id: &'a str,
    pub prepared: &'a Prepared<'graph>,
    pub inputs: PreparedBatch,
    pub timeout: Duration,
    pub transport: OutputTransport,
}
pub struct Configured {
    pub identity: Identity,
    pub request_id: UnitId,
    pub configuration: loom_art_wire::Configuration,
    pub timeout: Duration,
    pub transport: OutputTransport,
}
pub struct Pending {
    pub(crate) shader: Option<super::shader_prepare::Runtime>,
    pub(crate) dependencies: Vec<(UnitId, super::loom_input_stamp::SourceStamp)>,
    pub ticket: UnitId,
    pub(crate) topology: crate::workspace::TopologyStamp,
    pub(crate) selection: super::configuration_stamp::Stamp,
    pub plan: Configured,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Stale,
    InvalidTimeout,
    Allocation,
    Wire(crate::graph_execution_wire::Error),
}

/// 调用方在同一 UI 调度段存入空闲待执行槽，不得丢弃返回值后遗留注册请求。
/// 此处不联网；旧 worker 的取消由应用按注册代际退休时执行，使用其固定发现身份。
pub fn stage<'a>(
    registry: &mut Requests,
    plan: Plan<'_, '_>,
    mut lookup: impl FnMut(&UnitId) -> Option<&'a Editor>,
) -> Result<Execution, Error> {
    let configuration =
        loom_art_wire::Configuration::capture(plan.art_id, plan.prepared).map_err(Error::Wire)?;
    Configured {
        identity: plan.identity,
        request_id: plan.request_id,
        configuration,
        timeout: plan.timeout,
        transport: plan.transport,
    }
    .stage_checked(registry, plan.inputs, |unit, stamp| {
        lookup(unit).is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
    })
}
impl Configured {
    pub(crate) fn stage_checked(
        self,
        registry: &mut Requests,
        inputs: PreparedBatch,
        check: impl FnMut(&UnitId, &crate::desktop::loom_input_stamp::SourceStamp) -> bool,
    ) -> Result<Execution, Error> {
        if self.timeout.is_zero()
            || std::time::Instant::now()
                .checked_add(self.timeout)
                .is_none()
        {
            return Err(Error::InvalidTimeout);
        }
        if !inputs
            .freshness()
            .map_err(|_| Error::Allocation)?
            .is_current(check)
        {
            return Err(Error::Stale);
        }
        let mut descriptors = Vec::new();
        descriptors
            .try_reserve(inputs.descriptors().len())
            .map_err(|_| Error::Allocation)?;
        descriptors.extend(inputs.descriptors());
        let outgoing = loom_art_wire::stage_config(
            registry,
            &inputs.target,
            self.request_id,
            &self.configuration,
            &descriptors,
            self.transport,
        )
        .map_err(Error::Wire)?;
        drop(descriptors);
        Ok(Execution {
            shader: None,
            identity: self.identity,
            unit: inputs.target.clone(),
            request: outgoing.request,
            message: outgoing.execute,
            inputs,
            timeout: self.timeout,
        })
    }
}
