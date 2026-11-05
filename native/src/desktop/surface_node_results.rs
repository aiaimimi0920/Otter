//! Surface 节点输出替换与版本所有权；与普通 Loom 执行互斥，不执行资源描述符。
use super::*;
use crate::{surface_live_binding::Binding, surface_result_commit::Commit};
pub(super) struct Version {
    binding: Binding,
    revision: u64,
}
impl Results {
    /// 普通结果交付不能冒充重挂载：已有其他附件时须由挂载 owner 显式换绑。
    pub fn bind_surface_if_unbound(&mut self, binding: &Binding) -> Result<(), Error> {
        if self.surface_revoked
            || self
                .surface
                .as_ref()
                .is_some_and(|version| &version.binding != binding)
        {
            return Err(Error::Stale);
        }
        self.bind_surface(binding)
    }
    pub fn has_surface_result(&self, binding: &Binding, revision: u64) -> bool {
        self.surface
            .as_ref()
            .is_some_and(|version| &version.binding == binding && version.revision >= revision)
    }
    /// 仅在应用确认当前附件属于该 Art 节点后调用；重挂载保持旧输出直到新提交。
    pub fn bind_surface(&mut self, binding: &Binding) -> Result<(), Error> {
        if binding.node() != self.unit.as_str() || self.active.is_some() {
            return Err(Error::Stale);
        }
        if self
            .surface
            .as_ref()
            .is_some_and(|version| &version.binding == binding)
        {
            return Ok(());
        }
        self.surface = Some(Version {
            binding: binding.clone(),
            revision: 0,
        });
        self.surface_revoked = false;
        self.output_stamp = OutputStamp(Arc::new(()));
        Ok(())
    }
    pub fn surface_result_revision(&self) -> Option<u64> {
        self.surface.as_ref().map(|version| version.revision)
    }
    /// 授权/请求/场景审核由应用先完成；旧版本不变更，失败不消费原始 Commit。
    pub fn apply_surface_result(
        &mut self,
        binding: &Binding,
        commit: &Commit,
    ) -> Result<bool, Error> {
        let version = self.surface.as_ref().ok_or(Error::Stale)?;
        if self.active.is_some() || &version.binding != binding || !commit.matches_binding(binding)
        {
            return Err(Error::Stale);
        }
        if commit.revision() <= version.revision {
            return Ok(false);
        }
        let outputs = commit.output_values().map_err(|_| Error::InvalidOutput)?;
        let mut ports = Vec::new();
        ports
            .try_reserve_exact(outputs.len())
            .map_err(|_| Error::Allocation)?;
        for (name, value) in outputs {
            ports.push(Port {
                name,
                value: Output::Value(value),
            });
        }
        let stamp = OutputStamp(Arc::new(()));
        // 到此所有可失败工作已结束；Surface outputs 是完整替换，空 map 也须清旧端口。
        self.ports = ports;
        self.formal_outputs_committed = true;
        self.output_stamp = stamp;
        self.surface.as_mut().expect("checked surface").revision = commit.revision();
        self.status = Status::Completed;
        self.progress = Some(1.0);
        self.error = None;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn binding(instance: &str) -> Binding {
        Binding::from_mount(&serde_json::to_vec(&serde_json::json!({"instance":{
            "descriptor":{"instanceId":instance,"generation":1},"attachments":{"attachment:test":{
                "descriptor":{"attachmentId":"attachment:test","instanceId":instance,"hookNodeId":"node:test","deviceId":"viewer:test"},
                "lifecycle":"active","lifecycleRevision":1}}}})).unwrap(), "https://localhost:1234", "viewer:test", "node:test").unwrap()
    }
    fn commit(binding: &Binding, revision: u64, outputs: serde_json::Value) -> Commit {
        let scene = crate::surface_scene::Scene::parse(
            br#"{"id":"root","type":"view","events":{"click":"run"}}"#,
        )
        .unwrap();
        let mut event = crate::surface_click_route::ClickRoute::new(&scene, Some("root"))
            .next()
            .unwrap()
            .encode(binding, 1, Default::default())
            .unwrap();
        event.begin_submission();
        let ack = crate::surface_action_ack::Ack::parse(&serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":binding.instance(),"eventId":event.id(),
            "requestId":"request:test","accepted":true,"status":"succeeded"})).unwrap(), &event).unwrap();
        let raw = serde_json::value::RawValue::from_string(
            serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":binding.instance(),"generation":1,
            "requestId":"request:test","resultRevision":revision,"outputs":outputs})
            .to_string(),
        )
        .unwrap();
        Commit::parse_matching(&raw, &event, &ack).unwrap().unwrap()
    }
    #[test]
    fn surface_replaces_outputs_monotonically_and_empty_commit_clears_ports() {
        let binding = binding("instance:test");
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        results.bind_surface(&binding).unwrap();
        let first = commit(
            &binding,
            2,
            serde_json::json!({"old":{"kind":"value","value":false}}),
        );
        assert!(results.apply_surface_result(&binding, &first).unwrap());
        assert_eq!(results.save_value_outputs().unwrap()["old"], false);
        let stamp = results.output_stamp();
        assert!(!results.apply_surface_result(&binding, &first).unwrap());
        assert!(stamp.matches(&results));
        results.bind_surface(&binding).unwrap();
        assert_eq!(results.surface_result_revision(), Some(2));
        let empty = commit(&binding, 3, serde_json::json!({}));
        assert!(results.apply_surface_result(&binding, &empty).unwrap());
        assert!(results.ports().is_empty());
        assert_eq!(results.save_value_outputs(), Some(serde_json::Map::new()));
        assert!(!stamp.matches(&results));
        assert_eq!(results.status(), Status::Completed);
        assert_eq!(results.progress(), Some(1.0));
    }
    #[test]
    fn old_attachment_cannot_update_rebound_results() {
        let old = binding("instance:old");
        let new = binding("instance:new");
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        results.bind_surface(&old).unwrap();
        let old_commit = commit(&old, 99, serde_json::json!({}));
        results.bind_surface(&new).unwrap();
        assert_eq!(results.bind_surface_if_unbound(&old), Err(Error::Stale));
        let stamp = results.output_stamp();
        assert_eq!(
            results.apply_surface_result(&old, &old_commit),
            Err(Error::Stale)
        );
        assert_eq!(
            results.apply_surface_result(&new, &old_commit),
            Err(Error::Stale)
        );
        assert!(stamp.matches(&results));
        assert_eq!(results.surface_result_revision(), Some(0));
        assert!(Results::new(UnitId::parse("node:other").unwrap())
            .bind_surface(&new)
            .is_err());
    }
    #[test]
    fn normal_loom_execution_revokes_surface_writer_without_clearing_prior_outputs() {
        let binding = binding("instance:test");
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        results.bind_surface(&binding).unwrap();
        let commit = commit(
            &binding,
            1,
            serde_json::json!({"output":{"kind":"value","value":0}}),
        );
        results.apply_surface_result(&binding, &commit).unwrap();
        let stamp = results.output_stamp();
        let request = Request {
            id: UnitId::parse("request:loom").unwrap(),
            generation: 1,
        };
        results.begin(request.clone());
        assert_eq!(results.surface_result_revision(), None);
        assert_eq!(results.bind_surface(&binding), Err(Error::Stale));
        assert_eq!(results.bind_surface_if_unbound(&binding), Err(Error::Stale));
        assert_eq!(
            results.apply_surface_result(&binding, &commit),
            Err(Error::Stale)
        );
        assert_eq!(results.active(), Some(&request));
        assert_eq!(results.ports().len(), 1);
        assert!(stamp.matches(&results));
        assert!(results.cancel(&request));
        assert_eq!(results.bind_surface_if_unbound(&binding), Err(Error::Stale));
    }
}
