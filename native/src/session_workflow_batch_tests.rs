use super::*;
use serde_json::{json, Value};

fn fixture() -> (Vec<u8>, Snapshot) {
    let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[
        {"id":"a","x":0,"y":0,"w":20,"h":10,"originWorkflowId":"origin-a","originNodeId":"remote-a"},
        {"id":"b","x":0,"y":0,"w":20,"h":10,"originWorkflowId":"origin-b","originNodeId":"remote-b"}
    ],"links":[]})).unwrap();
    let snapshot = super::super::prepare(&saved, 1, &AtomicBool::new(false), |_, _| {
        Ok(super::super::Content {
            parameters: Default::default(),
            image: Default::default(),
        })
    })
    .unwrap();
    (saved, snapshot)
}
#[test]
fn lazy_batch_reuses_pending_identity_and_releases_each_message_on_ack() {
    let (saved, snapshot) = fixture();
    let cancel = AtomicBool::new(false);
    let budget = Budget::new(4096);
    let mut batch = Batch::prepare(&snapshot, &saved, &cancel).unwrap();
    assert_eq!(budget.used(), 0);
    assert_eq!(batch.remaining(), 3);
    let mut ids = std::collections::HashSet::new();
    let mut workflows = Vec::new();
    while batch.remaining() > 0 {
        let request = batch.current(&budget, 4096, &cancel).unwrap().unwrap();
        assert_eq!(budget.used(), 4096);
        let id = request.id().to_owned();
        assert!(uuid::Uuid::parse_str(id.strip_prefix("workflow-sync:").unwrap()).is_ok());
        assert!(ids.insert(id.clone()));
        let pointer = request.bytes().as_ptr();
        let value: Value = serde_json::from_slice(request.bytes()).unwrap();
        assert_eq!(value["params"]["requestId"], id);
        assert_eq!(value["params"]["workflowId"], request.workflow_id());
        workflows.push(request.workflow_id().to_owned());
        assert!(!batch.acknowledge("wrong"));
        let retry = batch.current(&budget, 4096, &cancel).unwrap().unwrap();
        assert_eq!(retry.id(), id);
        assert_eq!(retry.bytes().as_ptr(), pointer);
        assert!(batch.acknowledge(&id));
        assert!(!batch.acknowledge(&id));
        assert_eq!(budget.used(), 0);
    }
    assert_eq!(workflows[0], WORKFLOW_ID);
    workflows[1..].sort();
    assert_eq!(&workflows[1..], &["origin-a", "origin-b"]);
    assert!(batch.current(&budget, 4096, &cancel).unwrap().is_none());
}
#[test]
fn cancellation_budget_failure_and_source_mismatch_never_advance() {
    let (saved, snapshot) = fixture();
    let cancel = AtomicBool::new(false);
    assert!(Batch::prepare(&snapshot, b"{}", &cancel).is_err());
    let budget = Budget::new(4096);
    let mut batch = Batch::prepare(&snapshot, &saved, &cancel).unwrap();
    assert!(matches!(
        batch.current(&budget, 4097, &cancel),
        Err(Error::Budget)
    ));
    assert!(matches!(
        batch.current(&budget, 1, &cancel),
        Err(Error::Budget)
    ));
    assert_eq!(budget.used(), 0);
    assert_eq!(batch.remaining(), 3);
    batch.current(&budget, 4096, &cancel).unwrap();
    assert!(matches!(
        batch.current(&budget, 1, &cancel),
        Err(Error::Budget)
    ));
    assert!(matches!(
        batch.current(&budget, 4096, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    assert_eq!(batch.remaining(), 3);
    drop(batch);
    assert_eq!(budget.used(), 0);
}

#[test]
fn later_revisions_keep_images_in_global_and_origin_replacement_snapshots() {
    let cancel = AtomicBool::new(false);
    let budget = Budget::new(4096);
    let mut requests = std::collections::HashSet::new();
    for revision in [1, 2] {
        // 只改变几何/revision，图片引用不变；此为 wire 合同，不读取夹具路径。
        let saved = serde_json::to_vec(&json!({"documentRevision":revision,"stickers":[{
            "id":"local","x":revision,"y":0,"w":20,"h":10,
            "originWorkflowId":"origin","originNodeId":"remote",
            "src":"C:/archive/source.png","previewSrc":"C:/archive/preview.png"
        }],"links":[]}))
        .unwrap();
        let snapshot = super::super::content::prepare(&saved, revision, &[], &cancel, |_, _, _| {
            panic!("Unedited image must use direct payload")
        })
        .unwrap();
        let mut batch = Batch::prepare(&snapshot, &saved, &cancel).unwrap();
        for workflow in [WORKFLOW_ID, "origin"] {
            let request = batch.current(&budget, 4096, &cancel).unwrap().unwrap();
            assert_eq!(request.workflow_id(), workflow);
            let id = request.id().to_owned();
            assert!(requests.insert(id.clone()));
            let value: Value = serde_json::from_slice(request.bytes()).unwrap();
            let snapshot = &value["params"]["snapshot"];
            assert_eq!(snapshot["documentRevision"], revision);
            let node = &snapshot["nodes"][0];
            assert_eq!(
                node["id"],
                if workflow == WORKFLOW_ID {
                    "local"
                } else {
                    "remote"
                }
            );
            assert_eq!(node["data"]["src"], "C:/archive/source.png");
            assert_eq!(node["data"]["previewSrc"], "C:/archive/preview.png");
            assert!(node["data"]
                .as_object()
                .unwrap()
                .contains_key("rasterizedAnnotationLayerSrc"));
            assert!(batch.acknowledge(&id));
            assert_eq!(budget.used(), 0);
        }
        assert_eq!(batch.remaining(), 0);
    }
}
