//! scene接纳与资源步进的独立oracle；不依赖HTTP时序，明确保留、失效和未就绪边界。
use super::*;
use serde_json::json;
fn state(job: &Job, revision: u64) -> State {
    let b = Binding::from(&job.key);
    let value = json!({"protocolVersion":"loom.wall.v1","view":{"binding":b,"instanceId":"art","attachmentId":"view"},"width":64,"height":64,"generation":1,"sequence":0,
        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"art","attachmentId":"view","artId":"fixture","artVersion":"1","revision":revision,"runtime":"declarative","scene":{"id":"root","type":"view"},"resources":[],"resourceLeases":[]},"preview":null,"result":null,"confirmations":[],"pending":[],"failure":null});
    State::parse(&serde_json::to_vec(&value).unwrap(), &b, "art", None).unwrap()
}
#[test]
fn accepting_a_new_snapshot_clears_old_pixels_before_any_new_resource_is_ready() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key.clone(),
        layout: frame.layout.clone(),
        size: frame.size,
    };
    let mut entries = BTreeMap::new();
    assert!(admit(&mut entries, "art".into(), state(&job, 1), &shared, &job).is_none());
    shared.publish(1, frame);
    let displayed = shared.lock().frame.clone().unwrap();
    shared.confirm(&displayed, Instant::now());
    assert!(shared.lock().presented_frame.is_some());
    assert!(admit(&mut entries, "art".into(), state(&job, 2), &shared, &job).is_none());
    assert_eq!(entries["art"].state.snapshot.revision, 2);
    assert!(entries["art"].ready.is_none());
    let s = shared.lock();
    assert!(s.frame.is_none() && s.presented_frame.is_none() && s.presented.is_none());
    assert_eq!(s.status, "正在准备Art内容");
}
#[test]
fn late_state_during_freeze_preserves_the_exact_ready_owner_and_new_open_is_returned_to_close() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    let mut entries = BTreeMap::new();
    assert!(admit(&mut entries, "art".into(), state(&job, 1), &shared, &job).is_none());
    let first = Arc::new(Art {
        view: entries["art"].state.view.clone(),
        generation: entries["art"].state.generation,
        snapshot: entries["art"].state.snapshot.clone(),
        size: [64, 64],
        images: BTreeMap::new(),
    });
    entries.get_mut("art").unwrap().ready = Some(first.clone());
    shared.lock().frozen = true;
    assert_eq!(
        admit(&mut entries, "art".into(), state(&job, 2), &shared, &job)
            .unwrap()
            .snapshot
            .revision,
        2
    );
    assert_eq!(entries["art"].state.snapshot.revision, 1);
    assert!(Arc::ptr_eq(entries["art"].ready.as_ref().unwrap(), &first));
    assert!(admit(
        &mut BTreeMap::new(),
        "art".into(),
        state(&job, 1),
        &shared,
        &job
    )
    .is_some());
    shared.lock().generation += 1;
    assert!(admit(&mut entries, "art".into(), state(&job, 3), &shared, &job).is_some());
    assert_eq!(entries["art"].state.snapshot.revision, 1);
}
#[test]
fn image_steps_are_single_and_each_declared_dimension_is_checked_independently() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    let mut s = state(&job, 1);
    let snapshot = Arc::get_mut(&mut s.snapshot).unwrap();
    snapshot.dimensions.insert("a".into(), [Some(2), None]);
    snapshot.dimensions.insert("b".into(), [None, Some(1)]);
    let mut entries = BTreeMap::new();
    let _ = admit(&mut entries, "art".into(), s, &shared, &job);
    let image = Image {
        width: 2,
        height: 1,
        pixels: vec![0; 8].into(),
    };
    assert!(dimensions_match(&[Some(2), None], &image));
    assert!(!dimensions_match(&[Some(1), None], &image));
    assert!(!dimensions_match(&[None, Some(2)], &image));
    let mut images = BTreeMap::new();
    assert!(
        matches!(task(&entries, &images, &["art".into()]), Some(Task::Image(a,b)) if a=="art" && b=="a")
    );
    images.insert("a".into(), image.clone());
    assert!(matches!(task(&entries, &images, &["art".into()]), Some(Task::Image(_,b)) if b=="b"));
    images.insert("b".into(), image);
    assert!(task(&entries, &images, &["art".into()]).is_none());
}
