//! 归档引用安装为下一修订会话候选；复用预算化编码，不执行文件写入。
use super::*;
use crate::session_document::workflow_snapshot::metadata::{archive::Asset, Adoption};
use serde::{
    de::{IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

// Materialize only values that may survive the archive rewrite. Keep serde_json's
// Value semantics for unchanged fields (including historical unknown fields).
struct KeptSession(Map<String, Value>);
impl<'de> Deserialize<'de> for KeptSession {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Root;
        impl<'de> Visitor<'de> for Root {
            type Value = KeptSession;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an archived session object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<KeptSession, A::Error> {
                let mut root = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "stickers" => {
                            let stickers = map.next_value::<Vec<KeptSticker>>()?;
                            root.insert(
                                key,
                                Value::Array(stickers.into_iter().map(|s| s.0).collect()),
                            );
                        }
                        // The index was already decoded and merged by the archive owner.
                        "workflowAssetArchiveIndex" => {
                            map.next_value::<IgnoredAny>()?;
                        }
                        _ => {
                            root.insert(key, map.next_value()?);
                        }
                    }
                }
                Ok(KeptSession(root))
            }
        }
        decoder.deserialize_map(Root)
    }
}

struct KeptSticker(Value);
impl<'de> Deserialize<'de> for KeptSticker {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Sticker;
        impl<'de> Visitor<'de> for Sticker {
            type Value = KeptSticker;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sticker object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<KeptSticker, A::Error> {
                let mut fields = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if matches!(key.as_str(), "src" | "previewSrc") {
                        map.next_value::<IgnoredAny>()?;
                    } else {
                        fields.insert(key, map.next_value()?);
                    }
                }
                Ok(KeptSticker(Value::Object(fields)))
            }
        }
        decoder.deserialize_map(Sticker)
    }
}

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
    let KeptSession(mut root) = serde_json::from_slice(original).map_err(|_| Error::Source)?;
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
        }
    }
    if !replacements.is_empty() || !origins.is_empty() || !index.is_object() {
        return Err(Error::Window);
    }
    root.insert("workflowAssetArchiveIndex".into(), index);
    root.insert("documentSchemaVersion".into(), json!(1));
    root.insert("documentRevision".into(), json!(revision));
    let bytes = encode_bounded(&root, cancel)?;
    drop(root);
    let checked = decode(&bytes).map_err(|_| Error::Window)?;
    if checked.revision != revision || Topology::capture(&checked.workspace) != topology {
        return Err(Error::Topology);
    }
    Ok(Candidate {
        bytes,
        expected_revision,
        revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_rewrite_skips_replaced_images_but_round_trips_other_fields() {
        let inline = format!("data:image/png;base64,{}", "A".repeat(512 * 1024));
        let mut original = json!({
            "documentRevision":4,
            "stickers":[{"id":"s","x":0,"y":0,"w":1,"h":1,"src":inline,
                "previewSrc":inline,"imageEditState":{"raster":inline}}],
            "links":[], "workflowAssetArchiveIndex":{"version":1}
        });
        for i in 0..260 {
            original["stickers"][0][format!("future{i}")] = json!([1, true, "a\nb"]);
        }
        let saved = serde_json::to_vec(&original).unwrap();
        let KeptSession(kept) = serde_json::from_slice(&saved).unwrap();
        assert!(kept["stickers"][0].get("src").is_none());
        assert!(kept["stickers"][0].get("previewSrc").is_none());
        assert!(kept.get("workflowAssetArchiveIndex").is_none());
        assert_eq!(kept["stickers"][0]["imageEditState"]["raster"], inline);

        let rewritten = prepare(
            &saved,
            json!({"version":1,"workflows":{}}),
            &[Asset {
                sticker_id: "s",
                src: "new.png",
                preview_src: None,
            }],
            &[],
            &AtomicBool::new(false),
        )
        .unwrap();
        original["documentRevision"] = json!(5);
        original["documentSchemaVersion"] = json!(1);
        original["stickers"][0]["src"] = json!("new.png");
        original["stickers"][0]
            .as_object_mut()
            .unwrap()
            .remove("previewSrc");
        original["workflowAssetArchiveIndex"] = json!({"version":1,"workflows":{}});
        assert_eq!(
            serde_json::from_slice::<Value>(&rewritten.bytes).unwrap(),
            original
        );
    }
}
