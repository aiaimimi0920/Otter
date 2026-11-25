//! Synthetic inline pixels verify bounded viewport caching and entry identity; paths/URLs gain no implicit read permission.
use super::*;
#[test]
fn previews_decode_bounded_viewport_ranges_and_bind_images_to_entry_identity() {
    let input = crate::desktop::loom_inline::prepare(
        2,
        1,
        [0, 0, 255, 255].repeat(2).into(),
        &crate::editor::Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("png")
    };
    let uri = format!("data:image/png;base64,{base64}");
    let records: Vec<_> = (0..257)
        .map(|n| {
            serde_json::json!({"entryId":format!("entry-{n}"),
        "sourceStickerId":format!("source-{n}"), "createdAt":"", "snapshot":{
            "id":format!("source-{n}"),"x":0,"y":0,"w":2,"h":1,
            "previewSrc":"https://127.0.0.1:1/not-authorized", "src":uri }})
        })
        .collect();
    let source: Source = serde_json::to_vec(&serde_json::json!({"recycleBin":records}))
        .unwrap()
        .into();
    let mut previews = Previews::default();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !previews.update(&source, 0, 240..255, 257) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let thumbnail = previews.image(0, "entry-256").unwrap();
    assert_eq!(thumbnail.pixels(), [0, 0, 255, 255].repeat(2));
    let mut batch = previews.batch(0);
    assert_eq!(
        batch.next().map(|(slot, id, _)| (slot, id)),
        Some((128, "entry-128"))
    );
    assert_eq!(
        batch.last().map(|(slot, id, _)| (slot, id)),
        Some((256, "entry-256"))
    );
    assert!(previews.image(1, "entry-256").is_none());
    assert!(previews.image(0, "entry-0").is_none());
    assert_eq!(previews.ready[0].as_ref().unwrap().entries.len(), 129);
    assert!(previews.ready[1].is_none());
    assert!(previews.job.is_none());
    assert!(!previews.update(&source, 0, 242..257, 257));
    assert!(previews.job.is_none());
    while !previews.update(&source, 0, 0..15, 257) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(previews.image(0, "entry-0").is_some());
    assert!(previews.image(0, "entry-256").is_none());
    assert_eq!(previews.ready[0].as_ref().unwrap().entries.len(), 192);
    assert!(!previews.update(&source, 0, 0..15, 257));
    assert!(!previews.update(&source, 0, 240..255, 257));
    assert!(!previews.update(&source, 0, 0..15, 257));
    while previews.job.is_some() {
        assert!(std::time::Instant::now() < deadline);
        assert!(!previews.update(&source, 0, 0..15, 257));
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(previews.image(0, "entry-0").is_some());
    assert!(image("https://127.0.0.1:1/forbidden", &AtomicBool::new(false)).is_none());
    assert!(image("C:/private.png", &AtomicBool::new(false)).is_none());
}

#[test]
fn previews_same_view_retries_after_spawn_error_and_panic() {
    let source: Source = b"{}".to_vec().into();
    let mut previews = Previews::default();
    assert!(!previews.update_with(&source, 0, 0..1, 1, |_, _, _, _, _| {
        Err(std::io::Error::other("test spawn failure"))
    }));
    assert!(previews.job.is_none());
    assert!(
        !previews.update_with(&source, 0, 0..1, 1, |slot, _, _, _, notify| {
            slot.start("test-preview", notify, |_| {
                panic!("test preview worker panic")
            })
        })
    );
    wait_for_finish(&previews);
    assert!(!previews.update(&source, 0, 0..1, 1));
    wait_for_finish(&previews);
    assert!(previews.update(&source, 0, 0..1, 1));
}

#[test]
fn previews_invalidated_worker_retires_before_replacement_without_publishing() {
    let source: Source = b"{}".to_vec().into();
    let mut previews = Previews::default();
    let (release, blocked) = std::sync::mpsc::channel();
    assert!(
        !previews.update_with(&source, 0, 0..1, 257, |slot, library, range, _, notify| {
            slot.start("test-preview", notify, move |cancel| {
                blocked
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                assert!(cancel.load(Ordering::Acquire));
                Images {
                    library,
                    range,
                    entries: Vec::new(),
                }
            })
        })
    );
    previews.invalidate();
    assert!(
        !previews.update_with(&source, 0, 0..1, 257, |_, _, _, _, _| {
            panic!("must not replace an active worker")
        })
    );
    release.send(()).unwrap();
    wait_for_finish(&previews);
    assert!(!previews.update(&source, 0, 240..255, 257));
    assert!(previews.ready[0].is_none());
    wait_for_finish(&previews);
    assert!(previews.update(&source, 0, 240..255, 257));
}

fn wait_for_finish(previews: &Previews) {
    let worker = &previews.job.as_ref().unwrap().worker;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !worker.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}
