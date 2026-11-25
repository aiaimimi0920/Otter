//! 账号投射场景关联只保留公开身份；保存记录本身不授予账号或网络权限。
use super::*;
use crate::{session_projection_scope::Scope, session_source::Source, workspace_id::UnitId};
#[path = "projection_account_scene_patch.rs"]
mod patch;
pub const FIELD: &str = "otterNativeAccountProjection";
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    schema: u32,
    scope: Scope,
    loom_origin: String,
    account_device: String,
    local_unit: String,
    envelope: Envelope,
    revision: u64,
    digest: String,
    stopped: bool,
}
impl Binding {
    pub fn sent(
        content: &Content,
        context: &Context,
        origin: &str,
        scope: Scope,
    ) -> Result<Self, Error> {
        if !content.envelope.matches_source(context) {
            return Err(Error::Identity);
        }
        let binding = Self {
            schema: 1,
            scope,
            loom_origin: origin.into(),
            account_device: context.device_id.clone(),
            local_unit: content.envelope.source_unit().into(),
            envelope: content.envelope.clone(),
            revision: content.revision,
            digest: content.digest.clone(),
            stopped: !content.linked,
        };
        binding.validate(binding.unit(), scope)?;
        Ok(binding)
    }
    pub fn is_source(&self) -> bool {
        self.account_device == self.envelope.source_device()
    }
    pub fn device(&self) -> &str {
        &self.account_device
    }
    pub fn follows(&self, content: &Content, context: &Context) -> Result<Self, Error> {
        if self.stopped
            || !self.envelope.matches_origin(context)
            || (self.is_source() && !self.envelope.matches_source(context))
            || self.account_device != context.device_id
            || self.envelope != content.envelope
            || content.revision < self.revision
            || (content.revision == self.revision && content.digest != self.digest)
            || (!self.is_source()
                && (content.receiver_device_id.as_deref() != Some(&self.account_device)
                    || content.receiver_unit_id.as_deref() != Some(self.unit())))
        {
            return Err(Error::Identity);
        }
        let mut next = self.clone();
        next.revision = content.revision;
        next.digest = content.digest.clone();
        next.stopped = !content.linked;
        next.validate(next.unit(), next.scope)?;
        Ok(next)
    }
    pub fn stop(&self) -> Self {
        let mut next = self.clone();
        next.stopped = true;
        next
    }
    /// 只有匹配本账号接收设备/本次 Unit 的真实回复才可构造接收关联；不按响应换目标。
    pub fn received(
        content: &Content,
        context: &Context,
        loom_origin: &str,
        unit: &UnitId,
        scope: Scope,
    ) -> Result<Self, Error> {
        if !content.envelope.matches_origin(context)
            || content.envelope.source_device() == context.device_id
            || content.receiver_device_id.as_deref() != Some(&context.device_id)
            || content.receiver_unit_id.as_deref() != Some(unit.as_str())
        {
            return Err(Error::Identity);
        }
        let binding = Self {
            schema: 1,
            scope,
            loom_origin: loom_origin.into(),
            account_device: context.device_id.clone(),
            local_unit: unit.as_str().into(),
            envelope: content.envelope.clone(),
            revision: content.revision,
            digest: content.digest.clone(),
            stopped: !content.linked,
        };
        binding.validate(unit.as_str(), scope)?;
        Ok(binding)
    }
    pub fn envelope(&self) -> &Envelope {
        &self.envelope
    }
    pub fn unit(&self) -> &str {
        &self.local_unit
    }
    pub fn origin(&self) -> &str {
        &self.loom_origin
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn stopped(&self) -> bool {
        self.stopped
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn matches_content(&self, content: &Content) -> bool {
        self.envelope == content.envelope
            && self.revision == content.revision
            && self.digest == content.digest
            && self.stopped != content.linked
            && content.receiver_device_id.as_deref() == Some(&self.account_device)
            && content.receiver_unit_id.as_deref() == Some(self.unit())
    }
    fn validate(&self, unit: &str, scope: Scope) -> Result<(), Error> {
        self.envelope.validate()?;
        if self.schema != 1
            || self.scope != scope
            || self.local_unit != unit
            || UnitId::parse(unit).is_none()
            || !envelope::origin(&self.loom_origin)
            || !envelope::device(&self.account_device)
            || (self.is_source() && self.unit() != self.envelope.source_unit())
            || !revision(self.revision)
            || !digest(&self.digest)
            || (self.revision == 1 && self.digest != self.envelope.digest())
        {
            return Err(Error::Response);
        }
        Ok(())
    }
    pub fn decode(text: &str, unit: &str, scope: Scope) -> Result<Self, Error> {
        if text.len() > 8192 {
            return Err(Error::Budget);
        }
        let value: Self = serde_json::from_str(text).map_err(|_| Error::Response)?;
        value.validate(unit, scope)?;
        Ok(value)
    }
    /// 在后台修改原文；不可变 Envelope/设备/Unit/Scope 与 revision 摘要不能被换绑或回退。
    pub fn replace(
        &self,
        source: &Source,
        previous: Option<&Self>,
        cancel: &AtomicBool,
    ) -> Result<Source, Error> {
        self.validate_transition(previous)?;
        patch::replace(source, self, previous, None, cancel)
    }
    fn validate_transition(&self, previous: Option<&Self>) -> Result<(), Error> {
        self.validate(self.unit(), self.scope)?;
        if let Some(old) = previous {
            if old.stopped
                && self.is_source()
                && old.is_source()
                && old.scope == self.scope
                && old.local_unit == self.local_unit
                && old.account_device == self.account_device
                && old.loom_origin == self.loom_origin
                && old.envelope != self.envelope
            {
                return Ok(());
            }
            if old.envelope != self.envelope
                || old.scope != self.scope
                || old.account_device != self.account_device
                || old.local_unit != self.local_unit
                || old.loom_origin != self.loom_origin
                || old.stopped
                || self.revision < old.revision
                || (self.revision == old.revision && self.digest != old.digest)
            {
                return Err(Error::Response);
            }
        }
        Ok(())
    }
    /// 接收像素与关联在同一候选原文中交换，保留位置、裁剪、标注和未知字段。
    pub fn replace_image(
        &self,
        source: &Source,
        previous: &Self,
        png: &str,
        cancel: &AtomicBool,
    ) -> Result<Source, Error> {
        self.validate_transition(Some(previous))?;
        if self.is_source() {
            return Err(Error::Input);
        }
        patch::replace(source, self, Some(previous), Some(png), cancel)
    }
}
/// 一次后台解析的只读包含索引；绑定原 allocation，不能代替真实磁盘提交。
pub struct Index {
    source: Source,
    bindings: Vec<Binding>,
}
impl Index {
    pub fn capture(source: &Source) -> Result<Self, Error> {
        let parsed = source.parse().map_err(|_| Error::Response)?;
        let proof = crate::session_projection_scope::Proof::capture_parsed(&parsed);
        Self::capture_parsed(&parsed, proof.as_ref())
    }
    pub(crate) fn capture_parsed(
        parsed: &crate::session_source::Parsed<'_>,
        proof: Option<&crate::session_projection_scope::Proof>,
    ) -> Result<Self, Error> {
        let source = parsed.source();
        let scope = proof
            .filter(|proof| proof.is_for(source))
            .ok_or(Error::Response)?
            .scope();
        let root: std::collections::BTreeMap<&str, &serde_json::value::RawValue> =
            serde_json::from_slice(source).map_err(|_| Error::Response)?;
        let rows: Vec<&serde_json::value::RawValue> =
            serde_json::from_str(root.get("stickers").ok_or(Error::Response)?.get())
                .map_err(|_| Error::Response)?;
        let mut bindings = Vec::new();
        for row in rows {
            let fields: std::collections::BTreeMap<&str, &serde_json::value::RawValue> =
                serde_json::from_str(row.get()).map_err(|_| Error::Response)?;
            let Some(raw) = fields.get(FIELD).filter(|value| value.get() != "null") else {
                continue;
            };
            let unit: &str = serde_json::from_str(fields.get("id").ok_or(Error::Response)?.get())
                .map_err(|_| Error::Response)?;
            let binding = Binding::decode(raw.get(), unit, scope)?;
            if bindings
                .iter()
                .any(|old: &Binding| old.envelope == binding.envelope)
            {
                return Err(Error::Response);
            }
            bindings.push(binding);
        }
        if bindings.len() > parsed.document().workspace.units().len() {
            return Err(Error::Response);
        }
        Ok(Self {
            source: source.clone(),
            bindings,
        })
    }
    pub fn is_for(&self, source: &Source) -> bool {
        self.source.same_storage(source)
    }
    pub fn includes(&self, binding: &Binding) -> bool {
        self.bindings.contains(binding)
    }
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
}
