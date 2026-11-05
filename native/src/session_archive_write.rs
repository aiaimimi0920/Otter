//! 归档引用安装为下一修订会话候选；复用预算化编码，不执行文件写入。
use super::*;
use crate::session_document::workflow_snapshot::metadata::{archive::Asset, Adoption};

pub fn prepare(
    original: &[u8],
    index: Value,
    assets: &[Asset<'_>],
    adoptions: &[Adoption],
    cancel: &AtomicBool,
) -> Result<Candidate, Error> {
    check(cancel)?;
    let source = decode(original).map_err(|_| Error::Source)?;
    let expected_revision = source.revision;
    let topology = Topology::capture(&source.workspace);
    let revision = expected_revision.checked_add(1).ok_or(Error::Revision)?;
    drop(source);
    if assets.len() != topology.stickers.len() {
        return Err(Error::Window);
    }
    let mut replacements = HashMap::new();
    let mut origins = HashMap::new();
    if adoptions.len() > assets.len() {
        return Err(Error::Window);
    }
    for adoption in adoptions {
        check(cancel)?;
        if origins
            .insert(adoption.sticker_id.as_str(), adoption)
            .is_some()
        {
            return Err(Error::Window);
        }
    }
    for asset in assets {
        if replacements.insert(asset.sticker_id, asset).is_some() {
            return Err(Error::Window);
        }
    }
    let Value::Object(mut root) = serde_json::from_slice(original).map_err(|_| Error::Source)?
    else {
        return Err(Error::Source);
    };
    let stickers = root
        .get_mut("stickers")
        .and_then(Value::as_array_mut)
        .ok_or(Error::Window)?;
    for sticker in stickers {
        check(cancel)?;
        let fields = sticker.as_object_mut().ok_or(Error::Window)?;
        let id = fields
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::Window)?;
        let asset = replacements.remove(id).ok_or(Error::Window)?;
        // 来源采用与归档引用同 revision 提交；后续断开连线仍保留采用的工作流。
        if let Some(origin) = origins.remove(id) {
            fields.insert("originWorkflowId".into(), json!(origin.origin_workflow_id));
            fields.insert("originNodeId".into(), json!(origin.origin_node_id));
        }
        fields.insert("src".into(), json!(asset.src));
        if let Some(preview) = asset.preview_src {
            fields.insert("previewSrc".into(), json!(preview));
        } else {
            fields.remove("previewSrc");
        }
    }
    if !replacements.is_empty() || !origins.is_empty() || !index.is_object() {
        return Err(Error::Window);
    }
    root.insert("workflowAssetArchiveIndex".into(), index);
    root.insert("documentSchemaVersion".into(), json!(1));
    root.insert("documentRevision".into(), json!(revision));
    let mut output = Sink::new(measure(&root, cancel)?, cancel, crate::pixels::reserve)?;
    serde_json::to_writer(&mut output, &root).map_err(|_| failure(cancel))?;
    drop(root);
    check(cancel)?;
    let checked = decode(&output.bytes).map_err(|_| Error::Window)?;
    if checked.revision != revision || Topology::capture(&checked.workspace) != topology {
        return Err(Error::Topology);
    }
    Ok(Candidate {
        bytes: crate::pixels::Pixels::from_reserved(output.bytes, output.lease),
        expected_revision,
        revision,
    })
}
