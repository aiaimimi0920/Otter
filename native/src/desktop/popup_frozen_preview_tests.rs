//! 合成内联像素验证有界视口缓存和条目身份；路径或 URL 不取得隐式读取权限。
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
    assert!(previews.job.is_none());
    assert!(previews.image(0, "entry-0").is_some());
    assert!(image("https://127.0.0.1:1/forbidden", &AtomicBool::new(false)).is_none());
    assert!(image("C:/private.png", &AtomicBool::new(false)).is_none());
}
