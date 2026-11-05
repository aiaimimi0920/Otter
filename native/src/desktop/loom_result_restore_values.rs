//! 从会话恢复纯数据端口；不恢复执行状态、Surface owner 或远端资源访问权限。
use super::{Error, Output, OutputStamp, Port, Results};
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, sync::Arc};
#[path = "loom_result_restore_images.rs"]
mod images;

struct Ports(Vec<Port>);
impl<'de> Deserialize<'de> for Ports {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Ports;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object with at most 128 unique output ports")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Ports, M::Error> {
                let mut ports: Vec<Port> = Vec::new();
                while let Some(name) = map.next_key::<String>()? {
                    if ports.len() == 128
                        || name.is_empty()
                        || name.len() > 256
                        || name.chars().any(char::is_control)
                        || ports.iter().any(|port| port.name == name)
                    {
                        return Err(de::Error::custom("invalid or duplicate saved output port"));
                    }
                    ports.try_reserve(1).map_err(de::Error::custom)?;
                    ports.push(Port {
                        name,
                        value: Output::Value(map.next_value()?),
                    });
                }
                Ok(Ports(ports))
            }
        }
        decoder.deserialize_map(Entries)
    }
}

impl Results {
    /// 只用于尚未发布的新结果 owner；全部解析成功后安装，原有显示图保持不变。
    pub fn restore_value_outputs(
        &mut self,
        raw: &serde_json::value::RawValue,
    ) -> Result<(), Error> {
        self.restore_with(raw, |_| Ok(()))
    }
    /// 后台完整恢复入口：只物化内联图片；保留显式恢复预览优先级。
    pub fn restore_outputs(
        &mut self,
        raw: &serde_json::value::RawValue,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), Error> {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        self.restore_with(raw, |ports| images::decode(ports, cancel))?;
        if self.display.is_none() {
            self.display = self
                .ports
                .iter()
                .find(|port| port.name == "output_image" && port.value.image().is_some())
                .or_else(|| {
                    self.ports
                        .iter()
                        .find(|port| port.name == "output" && port.value.image().is_some())
                })
                .or_else(|| self.ports.iter().find(|port| port.value.image().is_some()))
                .and_then(|port| port.value.image().cloned());
            self.restored_preview_locked = self.display.is_some();
        }
        Ok(())
    }
    fn restore_with(
        &mut self,
        raw: &serde_json::value::RawValue,
        prepare: impl FnOnce(&mut [Port]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if self.active.is_some()
            || self.formal_outputs_committed
            || self.surface.is_some()
            || self.surface_revoked
        {
            return Err(Error::Stale);
        }
        if raw.get().len() > crate::session_document::MAX_SESSION_BYTES {
            return Err(Error::Budget);
        }
        let Ports(mut ports) = serde_json::from_str(raw.get()).map_err(|_| Error::InvalidOutput)?;
        prepare(&mut ports)?;
        self.ports = ports;
        self.formal_outputs_committed = true;
        self.output_stamp = OutputStamp(Arc::new(()));
        // 磁盘里的 outputs 不提供旧 Surface 写入授权；只有新挂载能显式注册。
        self.surface_revoked = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{desktop::loom_result_state::Status, workspace_id::UnitId};
    fn raw(text: &str) -> Box<serde_json::value::RawValue> {
        serde_json::value::RawValue::from_string(text.into()).unwrap()
    }
    #[test]
    fn restored_values_remain_idle_and_do_not_restore_runtime_authority() {
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        let before = results.output_stamp();
        results.restore_value_outputs(&raw(r#"{"zero":0,"false":false,"null":null,"resource":{"uri":"https://invalid.test/private","extra":true}}"#)).unwrap();
        assert!(!before.matches(&results));
        assert_eq!(results.status(), Status::Idle);
        assert!(results.active().is_none());
        assert!(results.surface_result_revision().is_none());
        assert!(results.surface_revoked);
        assert!(results.display().is_none());
        assert_eq!(results.save_value_outputs().unwrap()["zero"], 0);
        let stamp = results.output_stamp();
        assert_eq!(results.restore_value_outputs(&raw("{}")), Err(Error::Stale));
        assert!(stamp.matches(&results));
    }
    #[test]
    fn malformed_duplicate_or_excess_ports_cannot_partially_install() {
        let excess = serde_json::to_string(
            &(0..129)
                .map(|n| (format!("p{n}"), serde_json::json!(n)))
                .collect::<serde_json::Map<_, _>>(),
        )
        .unwrap();
        // 构造 JSON 对象时保留重复键的原文，以免 Map 提前合并。
        for text in ["[]", "{\"x\":1,\"x\":2}", "{\"\":1}", &excess] {
            let mut results = Results::new(UnitId::parse("node:test").unwrap());
            let stamp = results.output_stamp();
            assert!(results.restore_value_outputs(&raw(text)).is_err());
            assert!(stamp.matches(&results));
            assert!(results.save_value_outputs().is_none());
        }
    }
    #[test]
    fn active_or_previously_executed_owner_cannot_accept_disk_outputs() {
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        let request = crate::graph_requests::Request {
            id: UnitId::parse("request:test").unwrap(),
            generation: 1,
        };
        results.begin(request.clone());
        assert_eq!(results.restore_value_outputs(&raw("{}")), Err(Error::Stale));
        assert!(results.cancel(&request));
        assert_eq!(results.restore_value_outputs(&raw("{}")), Err(Error::Stale));
        assert!(results.save_value_outputs().is_none());
    }
}
