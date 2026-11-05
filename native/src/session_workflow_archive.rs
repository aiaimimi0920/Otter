//! 已发布图片引用的归档索引候选；不写资源或会话，不推进文档 revision。
use super::{check, Error, Plan};
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};

pub struct Asset<'a> {
    pub sticker_id: &'a str,
    pub src: &'a str,
    pub preview_src: Option<&'a str>,
}
/// 必须在资源发布成功后调用，传入处理后的引用；本函数不从路径推断读取权限。
/// 返回可嵌入下一次 session_write 候选的索引，不代表磁盘提交已完成。
pub fn prepare(
    plan: &Plan,
    saved: &[u8],
    assets: &[Asset<'_>],
    timestamp_ms: u64,
    cancel: &AtomicBool,
) -> Result<Value, Error> {
    check(cancel)?;
    if assets.len() > crate::workspace::MAX_UNITS {
        return Err(Error::Budget);
    }
    if !plan.matches(saved) {
        return Err(Error::Revision);
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Root {
        workflow_asset_archive_index: Option<Map<String, Value>>,
    }
    // 只反序列化索引，跳过会话中的大图片和其他字段，不构造第二份完整文档树。
    let root: Root = serde_json::from_slice(saved).map_err(|_| Error::Session)?;
    let mut index = root.workflow_asset_archive_index.unwrap_or_default();
    let version = index
        .get("version")
        .map(Value::as_u64)
        .unwrap_or(Some(0))
        .ok_or(Error::Session)?;
    if version > u32::MAX as u64 {
        return Err(Error::Session);
    }
    index.insert("version".into(), json!(version.max(1)));
    let mut workflows = match index.remove("workflows") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(workflows)) => workflows,
        _ => return Err(Error::Session),
    };
    let mut by_id = BTreeMap::new();
    for asset in assets {
        check(cancel)?;
        if crate::workspace_id::UnitId::parse(asset.sticker_id).is_none() {
            return Err(Error::Node);
        }
        if by_id.insert(asset.sticker_id, asset).is_some() {
            return Err(Error::Node);
        }
        for reference in [Some(asset.src), asset.preview_src].into_iter().flatten() {
            // Inline 内容必须先交资源 owner 发布，不能把大图片复制进每个工作流索引。
            if reference.starts_with("data:image")
                || reference.len() > 32 * 1024
                || reference.contains('\0')
            {
                return Err(Error::Content);
            }
        }
    }
    let timestamp = timestamp_ms.to_string();
    for (id, hint) in &plan.archive_hints().workflows {
        check(cancel)?;
        let mut nodes = Map::new();
        for (node, hint) in &hint.nodes {
            check(cancel)?;
            let Some(asset) = by_id.get(hint.sticker_id.as_str()) else {
                continue;
            };
            nodes.insert(
                node.clone(),
                json!({
                    "stickerId":asset.sticker_id, "updatedAt":timestamp,
                    "src": if asset.src.is_empty() {None} else {Some(asset.src)},
                    "previewSrc":asset.preview_src,
                }),
            );
        }
        // 与主干一致：命中工作流整体替换 nodes，未命中的旧工作流原样保留。
        workflows.insert(id.clone(), json!({"updatedAt":timestamp,"nodes":nodes}));
    }
    index.insert("workflows".into(), Value::Object(workflows));
    let value = Value::Object(index);
    // 统一编码上限覆盖旧索引与新增节点；不得将任意大的旧字段带进下一次保存。
    struct Count<'a> {
        remaining: usize,
        cancel: &'a AtomicBool,
    }
    impl std::io::Write for Count<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if check(self.cancel).is_err() || bytes.len() > self.remaining {
                return Err(std::io::Error::other(
                    "archive index cancelled or over budget",
                ));
            }
            self.remaining -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let result = serde_json::to_writer(
        Count {
            remaining: crate::session_document::MAX_SESSION_BYTES,
            cancel,
        },
        &value,
    );
    check(cancel)?;
    result.map_err(|_| Error::Budget)?;
    Ok(value)
}

#[cfg(test)]
#[path = "session_workflow_archive_tests.rs"]
mod tests;
