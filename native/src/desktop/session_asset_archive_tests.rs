use super::*;
use crate::session_document::workflow_snapshot::{self, Content};
use serde_json::json;
fn fixture(second: &str) -> (std::path::PathBuf, Root, Vec<u8>, Plan) {
    let directory =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-archive-batch-{}",
            crate::desktop::document_state::new_unit_id().unwrap()
        ));
    std::fs::create_dir(&directory).unwrap();
    let root = Root::open(&directory).unwrap();
    let image = crate::desktop::image::Image {
        width: 1,
        height: 1,
        pixels: vec![10, 20, 30, 255].into(),
    };
    let cancel = AtomicBool::new(false);
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let inline = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[
        {"id":"first","x":0,"y":0,"w":1,"h":1,"src":inline.as_str(),"previewSrc":inline.as_str(),"originWorkflowId":"origin"},
        {"id":"second","x":0,"y":0,"w":1,"h":1,"src":second}],"links":[],"future":{"keep":true}})).unwrap();
    let snapshot = workflow_snapshot::prepare(&saved, 1, &cancel, |_, _| {
        Ok(Content {
            parameters: Default::default(),
            image: Default::default(),
        })
    })
    .unwrap();
    let plan = workflow_snapshot::metadata::prepare(&snapshot, &saved, &cancel).unwrap();
    (directory, root, saved, plan)
}
#[test]
fn batch_publishes_both_slots_and_keeps_refs_pinned_with_matching_index() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, saved, plan) = fixture("https://example.invalid/not-fetched.png");
    let batch = prepare(&root, &plan, &saved, 123, &AtomicBool::new(false)).unwrap();
    assert!(batch.matches(&saved));
    assert!(!batch.matches(b"{}"));
    assert_eq!(batch.assets().len(), 2);
    let first = &batch.assets()[0];
    assert!(std::path::Path::new(&first.src).is_file());
    assert_ne!(Some(first.src.as_str()), first.preview_src.as_deref());
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .open(&first.src)
        .is_err());
    assert_eq!(
        batch.index()["workflows"]["hook-live"]["nodes"]["first"]["src"],
        first.src
    );
    assert_eq!(
        batch.assets()[1].src,
        "https://example.invalid/not-fetched.png"
    );
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
    let source_reference = first.src.clone();
    let prepared = batch
        .into_candidate(&saved, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(prepared.candidate().expected_revision, 1);
    assert_eq!(prepared.candidate().revision, 2);
    let document: serde_json::Value = serde_json::from_slice(&prepared.candidate().bytes).unwrap();
    assert_eq!(document["future"]["keep"], true);
    assert_eq!(document["stickers"][0]["src"], source_reference);
    assert_eq!(
        document["workflowAssetArchiveIndex"]["workflows"]["hook-live"]["nodes"]["first"]["src"],
        source_reference
    );
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .open(&source_reference)
        .is_err());
    let destination = directory.join("archived-session.json");
    crate::session_file_write::Staged::prepare(
        &destination,
        prepared.candidate(),
        &AtomicBool::new(false),
    )
    .unwrap()
    .commit_new(&AtomicBool::new(false))
    .unwrap();
    assert_eq!(
        std::fs::read(destination).unwrap(),
        prepared.candidate().bytes.as_ref()
    );
    drop((prepared, root));
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn failure_returns_no_batch_and_preserves_already_published_resources() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, saved, plan) = fixture("data:image/png;base64,broken");
    assert!(prepare(&root, &plan, &saved, 1, &AtomicBool::new(true)).is_err());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    assert!(prepare(&root, &plan, b"{}", 1, &AtomicBool::new(false)).is_err());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    assert!(prepare(&root, &plan, &saved, 1, &AtomicBool::new(false)).is_err());
    let paths: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(paths.len(), 2);
    for path in paths {
        assert!(std::fs::OpenOptions::new().write(true).open(path).is_ok());
    }
    drop(root);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn adopted_origin_is_committed_and_survives_disconnection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, saved, _) = fixture("");
    let cancel = AtomicBool::new(false);
    let mut document: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    document["links"] = json!([{"id":"edge","fromUnitId":"first","toUnitId":"second","fromPortId":"out","toPortId":"in"}]);
    document["stickers"][1]["future"] = json!({"keep":true});
    let saved = serde_json::to_vec(&document).unwrap();
    let make_snapshot = |bytes: &[u8], revision| {
        workflow_snapshot::prepare(bytes, revision, &cancel, |_, _| {
            Ok(Content {
                parameters: Default::default(),
                image: Default::default(),
            })
        })
        .unwrap()
    };
    let plan =
        workflow_snapshot::metadata::prepare(&make_snapshot(&saved, 1), &saved, &cancel).unwrap();
    assert_eq!(plan.adoptions().len(), 1);
    let prepared = prepare(&root, &plan, &saved, 123, &cancel)
        .unwrap()
        .into_candidate(&saved, &cancel)
        .unwrap();
    let path = directory.join("adopted.json");
    crate::session_file_write::Staged::prepare(&path, prepared.candidate(), &cancel)
        .unwrap()
        .commit_new(&cancel)
        .unwrap();
    let committed = std::fs::read(&path).unwrap();
    let document: serde_json::Value = serde_json::from_slice(&committed).unwrap();
    assert_eq!(document["stickers"][1]["originWorkflowId"], "origin");
    assert_eq!(document["stickers"][1]["originNodeId"], "second");
    assert_eq!(document["stickers"][1]["future"]["keep"], true);
    // 正式保存同样从新 Source 合并窗口字段；移除连线不能清除来源归属。
    let decoded = crate::session_document::decode(&committed).unwrap();
    let mut topology = crate::workspace_session::Topology::capture(&decoded.workspace);
    topology.links.clear();
    let disconnected =
        crate::session_write::assemble(&committed, &topology, &cancel, |_, fields| Ok(fields))
            .unwrap();
    let snapshot = make_snapshot(&disconnected.bytes, 3);
    let next =
        workflow_snapshot::metadata::prepare(&snapshot, &disconnected.bytes, &cancel).unwrap();
    assert!(next.adoptions().is_empty());
    assert_eq!(
        next.archive_hints().workflows["origin"].nodes["second"].sticker_id,
        "second"
    );
    drop((prepared, root));
    std::fs::remove_dir_all(directory).unwrap();
}
