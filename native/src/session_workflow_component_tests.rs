//! 连通分量来源选择、身份重映射和精确会话绑定。
use super::*;
use serde_json::json;
fn bytes(origins: &[Option<(&str, &str)>], links: &[(usize, usize)]) -> Vec<u8> {
    let nodes: Vec<_> = origins
        .iter()
        .enumerate()
        .map(|(i, origin)| {
            let mut node = json!({"id":format!("n{i}"),"type":"sticker","x":0,"y":0,"w":1,"h":1});
            if let Some((workflow, id)) = origin {
                node["originWorkflowId"] = (*workflow).into();
                node["originNodeId"] = (*id).into();
            }
            node
        })
        .collect();
    let links: Vec<_>=links.iter().enumerate().map(|(i,(a,b))|json!({"id":format!("e{i}"),"fromUnitId":format!("n{a}"),"toUnitId":format!("n{b}"),"fromPortId":"out","toPortId":"in"})).collect();
    serde_json::to_vec(&json!({"documentRevision":3,"stickers":nodes,"links":links})).unwrap()
}
fn global(saved: &[u8]) -> Snapshot {
    super::super::prepare(saved, 3, &AtomicBool::new(false), |_, _| {
        Ok(super::super::Content {
            parameters: Default::default(),
            image: Default::default(),
        })
    })
    .unwrap()
}
#[test]
fn majority_workflow_and_origin_ids_map_without_changing_global_snapshot() {
    let saved = bytes(
        &[
            Some(("wf-a", "a")),
            Some(("wf-b", "b")),
            Some(("wf-b", "c")),
            None,
        ],
        &[(0, 1), (1, 2), (2, 3)],
    );
    let snapshot = global(&saved);
    let parts = prepare(&snapshot, &saved, &AtomicBool::new(false)).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].workflow_id(), "wf-b");
    let value: Value = serde_json::from_slice(
        &parts[0]
            .encode_payload(8192, &AtomicBool::new(false))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["snapshot"]["nodes"][0]["id"], "a");
    assert_eq!(value["snapshot"]["nodes"][3]["id"], "n3");
    assert_eq!(value["snapshot"]["edges"][0]["source"], "a");
    assert_eq!(value["snapshot"]["edges"][0]["target"], "b");
    assert_eq!(snapshot.value()["nodes"][0]["id"], "n0");
    assert_eq!(value["snapshot"]["documentRevision"], 3);
}
#[test]
fn tied_votes_follow_bfs_order_and_disconnected_same_workflow_is_not_overwritten() {
    let saved = bytes(
        &[
            Some(("wf-a", "a")),
            Some(("wf-b", "b")),
            Some(("wf-a", "c")),
            None,
        ],
        &[(0, 1)],
    );
    let snapshot = global(&saved);
    let parts = prepare(&snapshot, &saved, &AtomicBool::new(false)).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].workflow_id(), "wf-a");
    assert_eq!(parts[0].node_count(), 3);
    let value: Value = serde_json::from_slice(
        &parts[0]
            .encode_payload(8192, &AtomicBool::new(false))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["snapshot"]["edges"].as_array().unwrap().len(), 1);
}
#[test]
fn collision_wrong_source_and_cancel_never_return_partial_component_plan() {
    let saved = bytes(&[Some(("wf", "duplicate")), Some(("wf", "duplicate"))], &[]);
    let snapshot = global(&saved);
    assert!(matches!(
        prepare(&snapshot, &saved, &AtomicBool::new(false)),
        Err(Error::Node)
    ));
    let other = bytes(&[None, None], &[]);
    assert!(matches!(
        prepare(&snapshot, &other, &AtomicBool::new(false)),
        Err(Error::Revision)
    ));
    assert!(matches!(
        prepare(&snapshot, &saved, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
}
