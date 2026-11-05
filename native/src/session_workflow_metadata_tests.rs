//! 主干 workflowAssetArchiveHints 形状与来源采用边界。
use super::*;
use serde_json::{json, Value};
fn source(empty: bool) -> Vec<u8> {
    let nodes = if empty {
        json!([])
    } else {
        json!([
            {"id":"a","x":0,"y":0,"w":1,"h":1,"originWorkflowId":"wf","originNodeId":"remote-a","src":"private-image"},
            {"id":"b","x":0,"y":0,"w":1,"h":1}
        ])
    };
    let links = if empty {
        json!([])
    } else {
        json!([{"id":"edge","fromUnitId":"a","toUnitId":"b","fromPortId":"out","toPortId":"in"}])
    };
    serde_json::to_vec(&json!({"documentRevision":4,"stickers":nodes,"links":links})).unwrap()
}
fn snapshot(saved: &[u8]) -> Snapshot {
    super::super::prepare(saved, 4, &AtomicBool::new(false), |_, _| {
        Ok(super::super::Content {
            parameters: Default::default(),
            image: Default::default(),
        })
    })
    .unwrap()
}
#[test]
fn archive_hints_include_global_and_remapped_nodes_without_resource_contents() {
    let saved = source(false);
    let plan = prepare(&snapshot(&saved), &saved, &AtomicBool::new(false)).unwrap();
    assert_eq!(plan.revision(), 4);
    assert!(plan.matches(&saved));
    assert_eq!(
        plan.adoptions(),
        [Adoption {
            sticker_id: "b".into(),
            origin_workflow_id: "wf".into(),
            origin_node_id: "b".into()
        }]
    );
    let value = serde_json::to_value(plan.archive_hints()).unwrap();
    assert_eq!(
        value["workflows"]["hook-live"]["nodes"]["a"],
        json!({"stickerId":"a"})
    );
    assert_eq!(
        value["workflows"]["wf"]["nodes"]["remote-a"],
        json!({"stickerId":"a"})
    );
    assert_eq!(
        value["workflows"]["wf"]["nodes"]["b"],
        json!({"stickerId":"b"})
    );
    assert!(!value.to_string().contains("private-image"));
    assert_eq!(source(false), saved);
}
#[test]
fn empty_global_hint_is_preserved_and_stale_source_or_cancel_is_rejected() {
    let saved = source(true);
    let snapshot = snapshot(&saved);
    let plan = prepare(&snapshot, &saved, &AtomicBool::new(false)).unwrap();
    assert!(plan.adoptions().is_empty());
    assert_eq!(
        serde_json::to_value(plan.archive_hints()).unwrap(),
        json!({"workflows":{"hook-live":{"nodes":{}}}})
    );
    let mut altered: Value = serde_json::from_slice(&saved).unwrap();
    altered["extension"] = true.into();
    let altered = serde_json::to_vec(&altered).unwrap();
    assert!(!plan.matches(&altered));
    assert!(matches!(
        prepare(&snapshot, &altered, &AtomicBool::new(false)),
        Err(Error::Revision)
    ));
    assert!(matches!(
        prepare(&snapshot, &saved, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
}
