//! 针对一个不可变目录整批解析 Art 候选；缺失能力保留身份，不生成可执行状态。
use crate::{
    loom_handshake::Handshake, session_art, session_document::Document, workspace_id::UnitId,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Entry {
    pub unit: UnitId,
    pub binding: Binding,
}
pub enum Binding {
    Ready {
        capability_index: usize,
        restored: session_art::Restored,
    },
    // 原始参数仍由完整会话候选拥有；不在无能力时猜测 secret 边界或丢弃未知值。
    Missing {
        art_id: Option<String>,
    },
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Identity,
    Restore {
        unit: String,
        cause: session_art::Error,
    },
}

/// 结果中的目录索引只属于本次 handshake；应用层必须绑定同一目录快照后才可安装。
/// 任意节点失败时整批局部结果释放，不修改已有窗口或交付部分可执行节点。
pub fn resolve(
    document: &Document<'_>,
    catalog: &Handshake,
    cancel: &AtomicBool,
) -> Result<Vec<Entry>, Error> {
    resolve_optional(document, Some(catalog), cancel)
}

pub fn resolve_optional(
    document: &Document<'_>,
    catalog: Option<&Handshake>,
    cancel: &AtomicBool,
) -> Result<Vec<Entry>, Error> {
    let mut entries = Vec::new();
    for window in &document.windows {
        check(cancel)?;
        if !window.is_art() {
            continue;
        }
        let unit = UnitId::parse(&window.id).ok_or(Error::Identity)?;
        let found = catalog.and_then(|catalog| {
            (0..catalog.arts().len())
                .filter_map(|index| {
                    catalog
                        .capability(index)
                        .map(|capability| (index, capability))
                })
                .find(|(_, capability)| window.art_id.as_deref() == Some(capability.id))
        });
        let binding = if let Some((capability_index, capability)) = found {
            let restored =
                session_art::restore(window, &capability).map_err(|cause| Error::Restore {
                    unit: window.id.clone(),
                    cause,
                })?;
            Binding::Ready {
                capability_index,
                restored,
            }
        } else {
            Binding::Missing {
                art_id: window.art_id.clone(),
            }
        };
        entries.push(Entry { unit, binding });
    }
    check(cancel)?;
    Ok(entries)
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> Handshake {
        let mut value = crate::loom_handshake::tests::fixture();
        value["capabilities"]["artDefinitions"] = serde_json::json!([
            {"id":"known","parameters":[{"id":"secret","secret":true}],"inputs":[],"outputs":[]}
        ]);
        crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
            .unwrap()
            .unwrap()
    }
    fn document(params: &str) -> Vec<u8> {
        format!(r#"{{"stickers":[
            {{"id":"a","type":"art","artId":"known","x":0,"y":0,"w":1,"h":1,"params":{{"legacy":42,"secret":"hidden"}}}},
            {{"id":"b","type":"art","artId":"missing","x":0,"y":0,"w":1,"h":1,"params":{params}}},
            {{"id":"s","x":0,"y":0,"w":1,"h":1}}],"links":[]}}"#).into_bytes()
    }
    #[test]
    fn mixed_session_resolves_known_and_retains_missing_without_guessing() {
        let bytes = document(r#"{"unknown":"kept"}"#);
        let document = crate::session_document::decode(&bytes).unwrap();
        let entries = resolve(&document, &catalog(), &AtomicBool::new(false)).unwrap();
        assert_eq!(entries.len(), 2);
        let Binding::Ready {
            capability_index,
            restored,
        } = &entries[0].binding
        else {
            panic!("known")
        };
        assert_eq!(*capability_index, 0);
        assert!(restored.parameters.entries().any(|(id, _)| id == "legacy"));
        assert!(!restored.parameters.entries().any(|(id, _)| id == "secret"));
        assert!(
            matches!(&entries[1].binding, Binding::Missing { art_id: Some(id) } if id == "missing")
        );
        assert_eq!(document.original_json(), bytes);
    }
    #[test]
    fn late_invalid_known_node_rejects_whole_batch_and_cancel_never_delivers() {
        let bytes = document("[]");
        let mut document = crate::session_document::decode(&bytes).unwrap();
        document.windows[1].art_id = Some("known".into());
        assert!(
            matches!(resolve(&document, &catalog(), &AtomicBool::new(false)),
            Err(Error::Restore { unit, .. }) if unit == "b")
        );
        assert!(matches!(
            resolve(&document, &catalog(), &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
        assert_eq!(document.original_json(), bytes);
    }
}
