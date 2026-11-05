//! 动作结果的身份与容量边界；保留有界原文，不执行 outputs/resource/statePatch。
use crate::{surface_action_ack::Ack, surface_click_event::ClickEvent};
use serde::Deserialize;
use serde_json::value::RawValue;
pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Protocol,
    Generation,
}
#[derive(Debug)]
pub struct Commit {
    revision: u64,
    raw: Box<RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire<'a> {
    protocol_version: &'a str,
    instance_id: &'a str,
    request_id: &'a str,
    generation: u64,
    result_revision: u64,
    #[serde(borrow)]
    outputs: &'a RawValue,
}
impl Commit {
    pub fn matches_binding(&self, binding: &crate::surface_live_binding::Binding) -> bool {
        serde_json::from_str::<Wire<'_>>(self.raw.get()).is_ok_and(|wire| {
            wire.instance_id == binding.instance() && wire.generation == binding.generation()
        })
    }
    /// 对齐前端 surfacePortValue：只解包描述符，不解析路径、下载资源或订阅流。
    /// 返回独立有界值用于节点原子更新；失败不消费原始提交证据。
    pub fn output_values(&self) -> Result<serde_json::Map<String, serde_json::Value>, Error> {
        output_values(&self.raw)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// 原文中的资源引用仍不可信；实际使用时须经过对应资源授权与类型校验。
    pub fn json(&self) -> &RawValue {
        &self.raw
    }
    pub fn outputs(&self) -> Result<Vec<crate::surface_result_ports::Port<'_>>, Error> {
        let wire: Wire<'_> = serde_json::from_str(self.raw.get()).map_err(|_| Error::Protocol)?;
        crate::surface_result_ports::parse(wire.outputs)
    }
    pub fn parse_matching(
        raw: &RawValue,
        event: &ClickEvent,
        ack: &Ack,
    ) -> Result<Option<Self>, Error> {
        if raw.get().len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let wire: Wire<'_> = serde_json::from_str(raw.get()).map_err(|_| Error::Protocol)?;
        if wire.instance_id != event.binding().instance() || wire.request_id != ack.request_id() {
            return Ok(None);
        }
        if wire.generation < event.binding().generation() {
            return Ok(None);
        }
        if wire.generation > event.binding().generation() {
            return Err(Error::Generation);
        }
        if !ack.matches_event(event)
            || wire.protocol_version != "loom.surface.v1"
            || wire.result_revision == 0
        {
            return Err(Error::Protocol);
        }
        crate::surface_result_ports::parse(wire.outputs)?;
        Ok(Some(Self {
            revision: wire.result_revision,
            raw: raw.to_owned(),
        }))
    }
}
#[path = "surface_result_values.rs"]
mod values;
use values::output_values;
