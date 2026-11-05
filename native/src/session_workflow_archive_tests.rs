use super::*;
use crate::session_document::workflow_snapshot::{self, Content};
fn fixture() -> (Vec<u8>, Plan) {
    let saved = serde_json::to_vec(&json!({"documentRevision":2,"stickers":[{"id":"local","x":0,"y":0,"w":1,"h":1,"originWorkflowId":"origin","originNodeId":"remote"}],"links":[],
        "workflowAssetArchiveIndex":{"version":0,"future":true,"workflows":{"keep":{"nodes":{"old":{"src":"old.png"}}},"origin":{"nodes":{"obsolete":{}}}}}})).unwrap();
    let snapshot = workflow_snapshot::prepare(&saved, 2, &AtomicBool::new(false), |_, _| {
        Ok(Content {
            parameters: Default::default(),
            image: Default::default(),
        })
    })
    .unwrap();
    let plan = super::super::prepare(&snapshot, &saved, &AtomicBool::new(false)).unwrap();
    (saved, plan)
}
#[test]
fn archive_replaces_selected_snapshots_and_retains_unmentioned_workflows() {
    let (saved, plan) = fixture();
    let index = prepare(
        &plan,
        &saved,
        &[Asset {
            sticker_id: "local",
            src: "images/source.png",
            preview_src: Some("images/preview.png"),
        }],
        123,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(index["version"], 1);
    assert_eq!(index["future"], true);
    assert_eq!(index["workflows"]["keep"]["nodes"]["old"]["src"], "old.png");
    assert!(index["workflows"]["origin"]["nodes"]
        .get("obsolete")
        .is_none());
    assert_eq!(
        index["workflows"]["origin"]["nodes"]["remote"]["stickerId"],
        "local"
    );
    assert_eq!(
        index["workflows"]["hook-live"]["nodes"]["local"]["src"],
        "images/source.png"
    );
    assert_eq!(index["workflows"]["origin"]["updatedAt"], "123");
    assert_eq!(
        serde_json::from_slice::<Value>(&saved).unwrap()["workflowAssetArchiveIndex"]["version"],
        0
    );
    let empty = prepare(&plan, &saved, &[], 456, &AtomicBool::new(false)).unwrap();
    assert_eq!(empty["workflows"]["origin"]["nodes"], json!({}));
}
#[test]
fn archive_rejects_stale_source_unpublished_inline_and_cancellation() {
    let (saved, plan) = fixture();
    assert_eq!(
        prepare(&plan, b"{}", &[], 1, &AtomicBool::new(false)),
        Err(Error::Revision)
    );
    assert_eq!(
        prepare(&plan, &saved, &[], 1, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
    assert_eq!(
        prepare(
            &plan,
            &saved,
            &[Asset {
                sticker_id: "local",
                src: "data:image/png;base64,AA==",
                preview_src: None
            }],
            1,
            &AtomicBool::new(false)
        ),
        Err(Error::Content)
    );
}
