//! 候选准备发生在执行worker；只用合成内联像素，不访问网络、路径或真实profile。
use super::*;
use serde_json::json;
fn png() -> String {
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
    format!("data:image/png;base64,{base64}")
}
#[test]
fn candidate_thumbnails_decode_real_pixels_and_preserve_order_indices_and_selected() {
    let value = json!({"items":[{"index":5,"title":"中文\n候选","imageUrl":png()},
        {"index":2,"thumbnail":png()}],"selectedIndex":2});
    let set = Set::prepare(Some(&value), &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    assert_eq!(
        (set.items[0].index, set.items[1].index, set.selected),
        (5, 2, Some(2))
    );
    assert_eq!(set.items[0].title, "中文 候选");
    let thumb = set.items[0].thumbnail.as_ref().unwrap();
    assert_eq!((thumb.width(), thumb.height()), (2, 1));
    assert_eq!(thumb.pixels(), [0, 0, 255, 255].repeat(2));
}
#[test]
fn paths_urls_and_malformed_previews_remain_selectable_without_ambient_io() {
    let value = json!({"items":[{"index":0,"imageUrl":"https://127.0.0.1:1/private"},
        {"index":1,"cachedThumbnailPath":"C:/Windows/win.ini","imageUrl":"file:///C:/Windows/win.ini"},
        {"index":2,"thumbnail":"data:image/png;base64,INVALID","imageUrl":png()}],"selectedIndex":20});
    let set = Set::prepare(Some(&value), &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    assert!(set.items[0].thumbnail.is_none() && set.items[1].thumbnail.is_none());
    assert!(set.items[2].thumbnail.is_some());
    assert_eq!(set.selected, None);
}
#[test]
fn candidate_capacity_duplicate_indices_and_cancel_reject_atomic_batch() {
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        Set::prepare(Some(&json!({"items":vec![Value::Null;65]})), &cancel),
        Err(Error::InvalidCandidates)
    ));
    assert!(matches!(
        Set::prepare(Some(&json!({"items":[{"index":1},{"index":1}]})), &cancel),
        Err(Error::InvalidCandidates)
    ));
    let value = json!({"items":[{"index":0},{"index":1}]});
    assert!(matches!(
        Set::prepare(Some(&value), &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    assert!(Set::prepare(
        Some(&json!({"items":[{"index":-1},{"index":1.2},{"index":4294967295u64}]})),
        &cancel
    )
    .unwrap()
    .is_none());
}
#[test]
fn thumbnail_budget_and_unsupported_codec_never_expand_input_or_pixels() {
    let cancel = AtomicBool::new(false);
    let mut remaining = 1;
    assert!(preview(&png(), &cancel, &mut remaining).is_none());
    let mut remaining = 16 * 1024 * 1024;
    assert!(preview("data:image/webp;base64,UklGRg==", &cancel, &mut remaining).is_none());
    let _com = crate::desktop::Apartment::new().unwrap();
    assert!(Image::from_candidate_thumbnail(b"RIFFxxxxWEBP", &|| false).is_err());
    assert!(Image::from_candidate_thumbnail(&[0; 1], &|| true).is_err());
}
