use super::*;
use crate::session_document::workflow_snapshot::{self, Content};
fn prepare(
    plan: &Plan,
    saved: &[u8],
    assets: &[Asset<'_>],
    time: u64,
    cancel: &AtomicBool,
) -> Result<Value, Error> {
    Source::read(plan, saved, cancel)?.index(assets, time, cancel)
}
fn fixture() -> (Vec<u8>, Plan) {
    let saved = serde_json::to_vec(&json!({"documentRevision":2,"stickers":[{"id":"local","x":0,"y":0,"w":1,"h":1,"originWorkflowId":"origin","originNodeId":"remote"}],"links":[],
        "workflowAssetArchiveIndex":{"version":0,"future":true,"workflows":{"keep":{"nodes":{"old":{"src":"old.png"}}},"origin":{"nodes":{"obsolete":{}}}}}})).unwrap();
    let plan = plan_for(&saved);
    (saved, plan)
}
fn plan_for(saved: &[u8]) -> Plan {
    let snapshot = workflow_snapshot::prepare(saved, 2, &AtomicBool::new(false), |_, _| {
        Ok(Content {
            parameters: Default::default(),
            image: Default::default(),
        })
    })
    .unwrap();
    super::super::prepare(&snapshot, saved, &AtomicBool::new(false)).unwrap()
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

#[test]
fn archive_view_borrows_unescaped_previews_and_defaults_null_index() {
    let saved = br#"{"documentRevision":2,"stickers":[
        {"id":"missing","x":0,"y":0,"w":1,"h":1},
        {"id":"empty","x":0,"y":0,"w":1,"h":1,"src":"","previewSrc":null},
        {"id":"with-preview","x":0,"y":0,"w":1,"h":1,"src":"source.png","previewSrc":"preview.png"}
    ],"links":[],"workflowAssetArchiveIndex":null}"#;
    let cancel = AtomicBool::new(false);
    let plan = plan_for(saved);
    let mut source = Source::read(&plan, saved, &cancel).unwrap();
    let stickers = source.take_stickers();
    assert_eq!(stickers[0].src, "");
    assert_eq!(stickers[1].src, "");
    assert!(stickers[0].preview_src.is_none());
    assert!(stickers[1].preview_src.is_none());
    assert!(matches!(stickers[2].src, Cow::Borrowed("source.png")));
    assert!(matches!(
        stickers[2].preview_src,
        Some(Cow::Borrowed("preview.png"))
    ));
    assert_eq!(source.index(&[], 1, &cancel).unwrap()["version"], 1);

    let mut changed = saved.to_vec();
    changed.push(b' ');
    assert_eq!(
        Source::read(&plan, &changed, &cancel).err(),
        Some(Error::Revision)
    );
}

#[test]
fn duplicate_archive_index_is_not_an_ambiguous_source() {
    let saved = br#"{"documentRevision":2,"stickers":[],"links":[],
        "workflowAssetArchiveIndex":null,"workflowAssetArchiveIndex":null}"#;
    let plan = plan_for(saved);
    assert_eq!(
        Source::read(&plan, saved, &AtomicBool::new(false)).err(),
        Some(Error::Session)
    );
}
