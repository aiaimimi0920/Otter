//! 原版列表合同/兼容JSON/有界图像与保数据事务；只使用linshi独占目录。
use super::*;
struct Com;
impl Com {
    fn new() -> Self {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
            .ok()
            .unwrap();
        }
        Self
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}
fn image(width: u32, height: u32) -> crate::image_decode::Image {
    crate::image_decode::Image {
        width,
        height,
        pixels: [10, 20, 30, 128].repeat((width * height) as usize).into(),
    }
}
fn root() -> std::path::PathBuf {
    let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
    let path = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
        .join(format!("otter-capture-history-tests-{id:?}"));
    std::fs::create_dir(&path).unwrap();
    path
}
#[test]
fn colors_deduplicate_refresh_and_cap_independently_of_screenshots() {
    let mut data = Data::default();
    for index in 0..30 {
        data.color(Color(index, 10, 20), u64::from(index));
    }
    assert_eq!(data.colors.len(), 24);
    data.color(Color(10, 10, 20), 99);
    assert_eq!(data.colors[0].hex, "#0a0a14");
    assert_eq!(data.colors[0].at, 99);
    assert_eq!(data.colors.len(), 24);
    assert!(data.screenshots.is_empty());
}
#[test]
fn same_screenshot_id_replaces_distinct_ids_coexist_and_remove_only_touches_history() {
    let mut data = Data::default();
    for index in 0..30 {
        data.screenshot(image::capture(index.to_string(), &image(1, 1), index).unwrap());
    }
    assert_eq!(data.screenshots.len(), 24);
    data.screenshot(image::capture("10".into(), &image(1, 1), 99).unwrap());
    assert_eq!(data.screenshots[0].id, "10");
    assert_eq!(data.screenshots[0].at, 99);
    assert!(data.remove("10"));
    assert!(!data.remove("10"));
    assert_eq!(data.screenshots.len(), 23);
}
#[test]
fn thumbnail_is_240_rounding_aspect_preserving_never_upscales_and_keeps_original_dimensions() {
    assert_eq!(image::size(1, 1), Some((1, 1)));
    assert_eq!(image::size(1920, 1080), Some((240, 135)));
    assert_eq!(image::size(1080, 1920), Some((135, 240)));
    assert_eq!(image::size(0, 1), None);
    let entry = image::capture("capture".into(), &image(480, 240), 1).unwrap();
    let preview = entry.preview.as_ref().unwrap();
    assert_eq!((entry.width, entry.height), (480, 240));
    assert_eq!((preview.image.width, preview.image.height), (240, 120));
    assert_eq!(&preview.image.pixels[..4], &[10, 20, 30, 128]);
    assert_eq!(preview.small.width(), 64);
    assert!(entry.thumbnail.starts_with("data:image/png;base64,"));
}
#[test]
fn legacy_json_restores_both_lists_sanitizes_bad_entries_and_preserves_extensions() {
    let _com = Com::new();
    let entry = image::capture("legacy".into(), &image(2, 1), 5).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"colors": [
        {"hex":" ABCDEF ","rgb":{"r":171,"g":205,"b":239},"at":1},
        {"hex":"invalid","rgb":{"r":1,"g":2,"b":3},"at":2}],
        "screenshots":[entry, {"id":"remote","thumbnail":"https://example/image","width":1,"height":1,"at":0}],
        "future":{"keep":true}})).unwrap();
    let data = Data::decode(&bytes).unwrap();
    assert_eq!(data.colors.len(), 1);
    assert_eq!(data.colors[0].hex, "#abcdef");
    assert_eq!(data.screenshots.len(), 1);
    assert_eq!(
        data.screenshots[0]
            .preview
            .as_ref()
            .unwrap()
            .image
            .pixels
            .as_ref(),
        &[10, 20, 30, 128, 10, 20, 30, 128]
    );
    assert_eq!(data.extra["future"]["keep"], true);
    assert!(Data::decode(b"not json").is_err());
    assert!(Data::decode(b"[]").is_err());
    assert!(Data::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
}
#[test]
fn late_startup_load_merges_new_commits_without_reversing_recency() {
    let mut disk = Data::default();
    disk.color(Color(1, 2, 3), 1);
    let mut recent = Data::default();
    recent.color(Color(4, 5, 6), 2);
    recent.color(Color(1, 2, 3), 3);
    disk.merge_recent(recent);
    assert_eq!(
        disk.colors.iter().map(|e| e.at).collect::<Vec<_>>(),
        vec![3, 2]
    );
}
#[test]
fn persistent_cache_reopens_excludes_second_writer_and_preserves_conflicting_or_bad_files() {
    let _com = Com::new();
    let root = root();
    let (mut store, mut data) = store::Store::open(&root).unwrap();
    assert!(store::Store::open(&root).is_err());
    data.color(Color(1, 2, 3), 1);
    store.save(&data).unwrap();
    data.screenshot(image::capture("id".into(), &image(1, 1), 2).unwrap());
    store.save(&data).unwrap();
    let retained = std::fs::read(root.join("history.json")).unwrap();
    std::fs::write(
        root.join("history.json"),
        b"{\"colors\":[],\"screenshots\":[],\"other\":1}",
    )
    .unwrap();
    let conflict = std::fs::read(root.join("history.json")).unwrap();
    assert!(store.save(&data).is_err());
    assert_eq!(std::fs::read(root.join("history.json")).unwrap(), conflict);
    drop(store);
    std::fs::write(root.join("history.json"), &retained).unwrap();
    let (store, restored) = store::Store::open(&root).unwrap();
    assert_eq!(restored.colors[0].hex, "#010203");
    assert_eq!(restored.screenshots[0].id, "id");
    drop(store);
    std::fs::write(root.join("history.json"), b"malformed").unwrap();
    assert!(store::Store::open(&root).is_err());
    assert_eq!(
        std::fs::read(root.join("history.json")).unwrap(),
        b"malformed"
    );
    // 留在linshi作为本次测试数据；不递归删除任何目录。
}
#[test]
fn embedded_only_image_policy_rejects_remote_traversal_and_invalid_payloads() {
    for url in [
        "https://example/image.png",
        "file:///C:/private.png",
        "data:image/svg+xml;base64,PHN2Zz4=",
        "data:image/png;base64,invalid!",
        "data:text/html;base64,AA==",
    ] {
        assert!(image::decode(url).is_err());
    }
}
#[test]
fn temporary_preview_failure_followed_by_new_color_save_preserves_existing_screenshot() {
    let _com = Com::new();
    let root = root();
    let mut original = Data::default();
    original.screenshot(image::capture("keep".into(), &image(2, 1), 1).unwrap());
    let bytes = serde_json::to_vec(&original).unwrap();
    std::fs::write(root.join("history.json"), &bytes).unwrap();
    let (mut store, _) = store::Store::open(&root).unwrap();
    let mut unavailable =
        Data::decode_with(&bytes, |_| Err("injected pixel budget exhaustion".into())).unwrap();
    assert_eq!(unavailable.screenshots.len(), 1);
    assert!(unavailable.screenshots[0].preview.is_none());
    assert_eq!(
        unavailable.screenshots[0].thumbnail,
        original.screenshots[0].thumbnail
    );
    unavailable.color(Color(7, 8, 9), 2);
    store.save(&unavailable).unwrap();
    drop(store);
    let (_, restored) = store::Store::open(&root).unwrap();
    assert_eq!(restored.screenshots[0].id, "keep");
    assert!(restored.screenshots[0].preview.is_some());
}
