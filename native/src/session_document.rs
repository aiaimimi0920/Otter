//! 旧会话只读入口：保留原始 JSON 借用，不把拓扑子集重编码成完整会话。
use crate::{
    workspace::{Workspace, MAX_LINKS, MAX_UNITS},
    workspace_session::{SessionGroup, SessionLink, Sticker, Topology},
};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, marker::PhantomData};
#[path = "session_workflow_snapshot.rs"]
pub mod workflow_snapshot;

pub const MAX_SESSION_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug)]
pub enum Error {
    Capacity,
    Json(serde_json::Error),
    FutureSchema(u32),
    Window,
    CoordinateSpace,
    Topology(crate::workspace_session::Error),
}
pub struct Document<'a> {
    original: &'a [u8],
    pub schema_version: u32,
    pub revision: u64,
    pub coordinate_space: Option<crate::session_placement::Space>,
    pub projection_scope: Option<crate::session_projection_scope::Scope>,
    pub workspace: Workspace,
    pub windows: Vec<crate::session_window::Window<'a>>,
}
impl<'a> Document<'a> {
    /// 原文只读引用；后续图片/编辑状态适配须读取原文，不能从拓扑切片推断完整数据。
    pub fn original_json(&self) -> &'a [u8] {
        self.original
    }
    /// 按需读取冻结条目；坏库不阻止普通窗口恢复，也不授权图片或文件读取。
    pub fn library(
        &self,
        kind: crate::session_library::Kind,
    ) -> Result<crate::session_library::Library<'a>, String> {
        crate::session_library::decode(self.original, kind)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a> {
    #[serde(default)]
    document_schema_version: u32,
    #[serde(default)]
    document_revision: u64,
    #[serde(default, deserialize_with = "crate::session_coordinates::optional")]
    otter_native_coordinate_space: Option<crate::session_coordinates::Metadata>,
    #[serde(
        default,
        deserialize_with = "crate::session_projection_scope::optional"
    )]
    otter_native_projection_scope: Option<crate::session_projection_scope::Scope>,
    #[serde(borrow, deserialize_with = "units")]
    stickers: Vec<crate::session_window::Window<'a>>,
    #[serde(deserialize_with = "links")]
    links: Vec<SessionLink>,
    #[serde(default, deserialize_with = "groups")]
    groups: Vec<SessionGroup>,
}
pub fn decode(bytes: &[u8]) -> Result<Document<'_>, Error> {
    if bytes.len() > MAX_SESSION_BYTES {
        return Err(Error::Capacity);
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(Error::Json)?;
    // 主干 schema 1，历史缺省 0；不能静默接受未来格式再以旧格式覆盖。
    if envelope.document_schema_version > 1 {
        return Err(Error::FutureSchema(envelope.document_schema_version));
    }
    if envelope.stickers.iter().any(|window| !window.valid()) {
        return Err(Error::Window);
    }
    let coordinate_space = envelope
        .otter_native_coordinate_space
        .as_ref()
        .map(|metadata| metadata.space().ok_or(Error::CoordinateSpace))
        .transpose()?;
    let topology = Topology {
        stickers: envelope
            .stickers
            .iter()
            .map(|window| Sticker {
                id: window.id.clone(),
                group_id: window.group_id.clone(),
            })
            .collect(),
        links: envelope.links,
        groups: envelope.groups,
    };
    let mut workspace = topology.restore().map_err(Error::Topology)?;
    for window in &envelope.stickers {
        if let Some(raw) = window.port_visibility {
            let visibility =
                crate::workspace::ports::Visibility::read(raw.get()).map_err(|_| Error::Window)?;
            let unit = crate::workspace_id::UnitId::parse(&window.id).ok_or(Error::Window)?;
            workspace
                .set_port_visibility(&unit, visibility)
                .map_err(|_| Error::Window)?;
        }
    }
    Ok(Document {
        original: bytes,
        schema_version: envelope.document_schema_version,
        revision: envelope.document_revision,
        coordinate_space,
        projection_scope: envelope.otter_native_projection_scope,
        workspace,
        windows: envelope.stickers,
    })
}

// 数组在反序列化阶段即限长，不能先无界分配 Vec 再交给模型拒绝。
pub(crate) fn bounded<'de, D: Deserializer<'de>, T: Deserialize<'de>, const LIMIT: usize>(
    decoder: D,
) -> Result<Vec<T>, D::Error> {
    struct Items<T, const N: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for Items<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "an array of at most {N} session records")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            for _ in 0..N {
                let Some(value) = seq.next_element()? else {
                    return Ok(values);
                };
                values.try_reserve(1).map_err(de::Error::custom)?;
                values.push(value);
            }
            // 超额项仅确认存在，不构建额外窗口/连线记录。
            if seq.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::custom("session record limit exceeded"));
            }
            Ok(values)
        }
    }
    decoder.deserialize_seq(Items::<T, LIMIT>(PhantomData))
}
fn units<'de: 'a, 'a, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<crate::session_window::Window<'a>>, D::Error> {
    bounded::<D, crate::session_window::Window<'a>, MAX_UNITS>(decoder)
}
fn links<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<SessionLink>, D::Error> {
    bounded::<D, SessionLink, MAX_LINKS>(decoder)
}
fn groups<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<SessionGroup>, D::Error> {
    bounded::<D, SessionGroup, MAX_UNITS>(decoder)
}

#[cfg(test)]
#[path = "session_document_tests.rs"]
mod tests;
