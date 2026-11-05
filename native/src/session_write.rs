//! 整会话内存候选：按身份合并，保留未知根字段；磁盘提交者必须校验 expected_revision。
#[path = "session_archive_write.rs"]
pub mod archive;
use crate::{
    session_document::{decode, MAX_SESSION_BYTES},
    workspace_session::{Sticker, Topology},
};
use serde_json::{json, Map, Value};
use std::{
    collections::HashMap,
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Source,
    Topology,
    Window,
    Revision,
    Budget,
}
pub struct Candidate {
    pub bytes: crate::pixels::Pixels,
    pub expected_revision: u64,
    pub revision: u64,
}

/// 窗口字段与新编码字符串的预算共同交付；字段先释放，租约后释放。
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct WindowCandidate {
    fields: Map<String, Value>,
    #[serde(skip)]
    leases: Vec<crate::byte_budget::Lease>,
}
impl WindowCandidate {
    #[cfg(windows)]
    pub(crate) fn write_projection_edit(
        &mut self,
        snapshot: Option<&crate::projection_edit::workspace::Snapshot>,
    ) -> Result<(), Error> {
        self.fields.remove(crate::projection_edit::workspace::FIELD);
        if let Some(snapshot) = snapshot {
            snapshot
                .install(&mut self.fields)
                .map_err(|_| Error::Window)?;
        }
        Ok(())
    }
    pub fn new(fields: Map<String, Value>) -> Self {
        Self {
            fields,
            leases: Vec::new(),
        }
    }
    /// 移动同一字符串 allocation，不能先释放其预算再交给 serde。
    pub fn insert_encoded(&mut self, key: &str, value: crate::loom_base64::Encoded) {
        let (text, lease) = value.into_parts();
        self.fields.insert(key.into(), Value::String(text));
        self.leases.push(lease);
    }
    /// 图片端口与标量端口合为同一对象，字符串预算保持到整个候选释放。
    pub fn insert_encoded_output(
        &mut self,
        key: &str,
        value: crate::loom_base64::Encoded,
    ) -> Result<(), Error> {
        let Some(Value::Object(outputs)) = self.fields.get_mut("outputs") else {
            return Err(Error::Window);
        };
        if key.is_empty()
            || key.len() > 256
            || key.chars().any(char::is_control)
            || outputs.contains_key(key)
            || outputs.len() >= 128
        {
            return Err(Error::Window);
        }
        self.leases.try_reserve(1).map_err(|_| Error::Budget)?;
        let (text, lease) = value.into_parts();
        outputs.insert(key.into(), Value::String(text));
        self.leases.push(lease);
        Ok(())
    }
}

/// build 消费旧窗口对象而不复制图片字符串；新对象收到空 Map，必须提供完整窗口内容。
/// 回调只构造内存，不允许发布副作用。错误、取消或回读失败均不交付部分候选。
pub fn assemble(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    mut build: impl FnMut(&Sticker, Map<String, Value>) -> Result<Map<String, Value>, Error>,
) -> Result<Candidate, Error> {
    assemble_owned(original, topology, cancel, |sticker, old| {
        build(sticker, old).map(WindowCandidate::new)
    })
}

/// 带预算的资源安装入口；所有窗口租约保持到完整 JSON 树释放后。
pub fn assemble_owned(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
) -> Result<Candidate, Error> {
    assemble_owned_with_space(original, topology, cancel, None, build)
}

/// 原生保存携带明确坐标空间；旧调用不猜测或改写来源空间。
pub fn assemble_owned_with_space(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    space: Option<crate::session_placement::Space>,
    build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
) -> Result<Candidate, Error> {
    assemble_owned_at_revision(original, topology, cancel, space, None, None, build)
}

/// 独立恢复检查点可有自己的磁盘修订；仍由同一组装/预算/回读 owner 编码一次。
pub(crate) fn assemble_owned_at_revision(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    space: Option<crate::session_placement::Space>,
    disk_revision: Option<u64>,
    projection_scope: Option<crate::session_projection_scope::Scope>,
    mut build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
) -> Result<Candidate, Error> {
    check(cancel)?;
    let source = decode(original).map_err(|_| Error::Source)?;
    let expected_revision = disk_revision.unwrap_or(source.revision);
    let revision = expected_revision.checked_add(1).ok_or(Error::Revision)?;
    topology.restore().map_err(|_| Error::Topology)?;
    drop(source);
    // 声明在 root 前面，错误提前返回时也先释放 JSON、再释放租约。
    let mut leases = Vec::new();
    let Value::Object(mut root) = serde_json::from_slice(original).map_err(|_| Error::Source)?
    else {
        return Err(Error::Source);
    };
    let mut windows = indexed(root.remove("stickers"))?;
    if let Some(scope) = projection_scope {
        scope.install(&mut root)?;
    }
    if let Some(space) = space {
        crate::session_coordinates::install(&mut root, space).map_err(|_| Error::Source)?;
    }
    let old_links = indexed(root.remove("links"))?;
    let old_groups = indexed(root.remove("groups"))?;
    let mut used = measure(&root, cancel)?;
    let mut stickers = Vec::with_capacity(topology.stickers.len());
    for sticker in &topology.stickers {
        check(cancel)?;
        let old = windows.remove(&sticker.id).unwrap_or_default();
        let WindowCandidate {
            fields: mut window,
            leases: window_leases,
        } = build(sticker, old)?;
        leases.extend(window_leases);
        // 身份/分组由已验证拓扑拥有；回调不能偷偷引入另一份对象或悬空分组。
        if window
            .get("id")
            .is_some_and(|id| id.as_str() != Some(&sticker.id))
        {
            return Err(Error::Window);
        }
        window.insert("id".into(), sticker.id.clone().into());
        match &sticker.group_id {
            Some(id) => {
                window.insert("groupId".into(), id.clone().into());
            }
            None => {
                window.remove("groupId");
            }
        }
        used = used
            .checked_add(measure(&window, cancel)?)
            .ok_or(Error::Budget)?;
        if used > MAX_SESSION_BYTES {
            return Err(Error::Budget);
        }
        stickers.push(Value::Object(window));
    }
    root.insert("stickers".into(), stickers.into());
    root.insert(
        "links".into(),
        merge_records(old_links, &topology.links, cancel)?.into(),
    );
    root.insert(
        "groups".into(),
        merge_records(old_groups, &topology.groups, cancel)?.into(),
    );
    root.insert("documentSchemaVersion".into(), json!(1));
    root.insert("documentRevision".into(), json!(revision));
    check(cancel)?;
    // 先计数并预留实际容量，避免序列化期间几何扩容绕过共享预算。
    let length = measure(&root, cancel)?;
    let mut output = Sink::new(length, cancel, crate::pixels::reserve)?;
    serde_json::to_writer(&mut output, &root).map_err(|_| failure(cancel))?;
    drop(root);
    drop(leases);
    check(cancel)?;
    let checked = decode(&output.bytes).map_err(|_| Error::Window)?;
    if checked.revision != revision || Topology::capture(&checked.workspace) != *topology {
        return Err(Error::Topology);
    }
    check(cancel)?;
    Ok(Candidate {
        bytes: crate::pixels::Pixels::from_reserved(output.bytes, output.lease),
        expected_revision,
        revision,
    })
}
/// 共用有界序列化；输出持有像素共享预算，失败不交付部分字节。
pub(crate) fn encode_bounded(
    value: &impl serde::Serialize,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, Error> {
    check(cancel)?;
    let length = measure(value, cancel)?;
    let mut output = Sink::new(length, cancel, crate::pixels::reserve)?;
    serde_json::to_writer(&mut output, value).map_err(|_| failure(cancel))?;
    check(cancel)?;
    Ok(crate::pixels::Pixels::from_reserved(
        output.bytes,
        output.lease,
    ))
}

fn indexed(value: Option<Value>) -> Result<HashMap<String, Map<String, Value>>, Error> {
    let items = match value {
        None => Vec::new(),
        Some(Value::Array(items)) => items,
        _ => return Err(Error::Source),
    };
    let mut map = HashMap::with_capacity(items.len());
    for item in items {
        let Value::Object(item) = item else {
            return Err(Error::Source);
        };
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::Source)?
            .to_owned();
        if map.insert(id, item).is_some() {
            return Err(Error::Source);
        }
    }
    Ok(map)
}
fn merge_records<T: serde::Serialize>(
    mut previous: HashMap<String, Map<String, Value>>,
    records: &[T],
    cancel: &AtomicBool,
) -> Result<Vec<Value>, Error> {
    let mut values = Vec::with_capacity(records.len());
    for record in records {
        check(cancel)?;
        let Value::Object(fields) = serde_json::to_value(record).map_err(|_| Error::Topology)?
        else {
            return Err(Error::Topology);
        };
        let id = fields
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::Topology)?;
        let mut value = previous.remove(id).unwrap_or_default();
        value.extend(fields);
        values.push(value.into());
    }
    Ok(values)
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn failure(cancel: &AtomicBool) -> Error {
    if cancel.load(Ordering::Acquire) {
        Error::Cancelled
    } else {
        Error::Budget
    }
}
fn measure(value: &impl serde::Serialize, cancel: &AtomicBool) -> Result<usize, Error> {
    struct Count<'a> {
        size: usize,
        cancel: &'a AtomicBool,
    }
    impl Write for Count<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.cancel.load(Ordering::Acquire)
                || bytes.len() > MAX_SESSION_BYTES.saturating_sub(self.size)
            {
                return Err(std::io::Error::other(
                    "Session candidate budget or cancellation",
                ));
            }
            self.size += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count { size: 0, cancel };
    serde_json::to_writer(&mut count, value).map_err(|_| failure(cancel))?;
    Ok(count.size)
}
struct Sink<'a> {
    bytes: Vec<u8>,
    lease: crate::byte_budget::Lease,
    cancel: &'a AtomicBool,
}
impl<'a> Sink<'a> {
    fn new(
        length: usize,
        cancel: &'a AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<Self, Error> {
        check(cancel)?;
        if length > MAX_SESSION_BYTES {
            return Err(Error::Budget);
        }
        let mut lease = reserve(length).ok_or(Error::Budget)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| Error::Budget)?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Budget);
        }
        check(cancel)?;
        Ok(Self {
            bytes,
            lease,
            cancel,
        })
    }
}
impl Write for Sink<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.cancel.load(Ordering::Acquire)
            || bytes.len() > MAX_SESSION_BYTES.saturating_sub(self.bytes.len())
            || bytes.len() > self.bytes.capacity().saturating_sub(self.bytes.len())
        {
            return Err(std::io::Error::other(
                "Session candidate budget or cancellation",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_write_tests.rs"]
mod tests;
