//! 同一排版、点击和单根选区；剪贴板/命令只是数据结果，不访问用户内容。
use super::*;
use serde_json::json;
fn view(local_copy: bool) -> Views {
    Views::prepare(vec![Visual {
        plugin:crate::extension_visual::OCR.into(),attachment:"ocr-result".into(),descriptor:"view".into(),
        bounds:Bounds::Fixed([4.,6.,180.,96.]),command:Some("copy-block".into()),local_copy,generation:17,revision:9,placeholder:false,
        scene:json!({"id":"root","type":"stack","layout":{"position":"relative","width":180,"height":96},
            "props":{"eventPayload":{"text":"ancestor text must not replace display text"}},
            "events":{"click":"copy-block"},"children":[
                {"id":"one","type":"text","props":{"text":"First\nSecond","selectable":true},
                    "layout":{"position":"absolute","left":0,"top":0,"width":180,"height":44},"style":{"whiteSpace":"pre"}},
                {"id":"two","type":"text","props":{"text":"Other block","selectable":true},
                    "layout":{"position":"absolute","left":0,"top":55,"width":180,"height":32}}
            ]})
    }], [192., 108.]).unwrap()
}
fn point(v: &Views, id: &str) -> [f32; 2] {
    let p = v.items[0].1.borrow();
    let item = p.text().items().iter().find(|t| t.node_id() == id).unwrap();
    let r = item.selection_rects(0..1).unwrap()[0];
    [r.left + 4.1, (r.top + r.bottom) / 2. + 6.]
}
#[test]
fn unit_relative_fallback_reflows_text_hits_and_offline_badges_never_take_input() {
    let mut c = crate::extension_visual::tests::catalog_value();
    c["contributions"]["renderers"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let c =
        crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&c).unwrap()).unwrap();
    let store = crate::extension_visual::availability_tests::text_store();
    let v = Views::prepare(
        crate::extension_visual::project(Some(&c), &store, 4),
        [64.; 2],
    )
    .unwrap();
    assert_eq!(v.rects()[0].right, 64.);
    assert!(v.hit([60., 12.]).is_some());
    v.resize([128., 64.]).unwrap();
    assert_eq!(v.rects()[0].right, 128.);
    let point = [120., 12.];
    assert!(v.begin(point, false).unwrap());
    let Outcome::Copy(text) = v.finish(point, Modifiers::default()).unwrap() else {
        panic!("not local fallback copy")
    };
    assert_eq!(text, "原文\n第二行");
    let offline = Views::prepare(
        crate::extension_visual::project(None, &store, 4),
        [128., 64.],
    )
    .unwrap();
    assert!(offline.rects().is_empty());
    assert!(offline.hit([8., 8.]).is_none());
    assert!(!offline.begin([8., 8.], false).unwrap());
}
#[test]
fn local_click_copies_only_the_displayed_text_node_with_original_newlines() {
    let v = view(true);
    let p = point(&v, "one");
    assert!(v.begin(p, false).unwrap());
    let Outcome::Copy(text) = v.finish(p, Modifiers::default()).unwrap() else {
        panic!("not local copy")
    };
    assert_eq!(text, "First\nSecond");
    assert!(!v.dragging());
    assert_eq!(v.selected_text().unwrap(), None);
}
#[test]
fn selectable_root_trailing_space_uses_display_text_not_ancestor_action_payload() {
    let v = view(true);
    let glyph = point(&v, "one");
    let blank = [170., glyph[1]];
    v.begin(blank, false).unwrap();
    let Outcome::Copy(text) = v.finish(blank, Modifiers::default()).unwrap() else {
        panic!("not local copy")
    };
    assert_eq!(text, "First\nSecond");
    v.begin(blank, false).unwrap();
    v.finish(glyph, Modifiers::default()).unwrap();
    assert_eq!(v.selected_text().unwrap().as_deref(), Some("First"));
}
#[test]
fn drag_is_locked_to_initial_text_root_and_never_fires_whole_block_copy_or_command() {
    let v = view(true);
    let first = point(&v, "one");
    let second = point(&v, "two");
    v.begin(first, false).unwrap();
    v.update(second).unwrap();
    assert!(matches!(
        v.finish([170., second[1]], Modifiers::default()).unwrap(),
        Outcome::None
    ));
    assert_eq!(v.selected_text().unwrap().as_deref(), Some("First\nSecond"));
    v.cancel(false);
    assert!(v.selected_text().unwrap().is_some());
    v.cancel(true);
    assert_eq!(v.selected_text().unwrap(), None);
}
#[test]
fn double_click_selects_word_and_does_not_invoke_parent_or_minify() {
    let v = view(true);
    let p = point(&v, "one");
    v.begin(p, true).unwrap();
    assert!(matches!(
        v.finish(p, Modifiers::default()).unwrap(),
        Outcome::None
    ));
    assert_eq!(v.selected_text().unwrap().as_deref(), Some("First"));
}
#[test]
fn nonlocal_surface_click_has_descriptor_generation_attachment_revision_and_exact_payload() {
    let v = view(false);
    let p = point(&v, "one");
    v.begin(p, false).unwrap();
    let Outcome::Command { command, input } = v
        .finish(
            p,
            Modifiers {
                ctrl_key: true,
                ..Default::default()
            },
        )
        .unwrap()
    else {
        panic!("not command")
    };
    assert_eq!(command, "copy-block");
    assert_eq!(
        input,
        json!({"surfaceEvent":{"attachmentId":"ocr-result","nodeId":"root","event":"click",
        "action":"copy-block","class":"discrete","generation":17,"baseRevision":9,
        "modifiers":{"ctrlKey":true,"altKey":false,"shiftKey":false,"metaKey":false},
        "payload":{"text":"ancestor text must not replace display text"}}})
    );
}
#[test]
fn cancelled_or_moved_press_cannot_execute_and_empty_overlay_space_does_not_hit() {
    let v = view(true);
    let p = point(&v, "one");
    v.begin(p, false).unwrap();
    v.cancel(false);
    assert!(matches!(
        v.finish(p, Modifiers::default()).unwrap(),
        Outcome::None
    ));
    // Remove ancestor click to distinguish selectable glyphs from the overlay bounds.
    let mut visual = v.items[0].0.scene.clone();
    visual.as_object_mut().unwrap().remove("events");
    let mut v = view(true);
    v.items[0].0.scene = visual.clone();
    v.items[0]
        .1
        .borrow_mut()
        .replace(
            crate::surface_scene::Scene::parse(&serde_json::to_vec(&visual).unwrap()).unwrap(),
            crate::surface_viewport::style(),
            crate::surface_viewport::context([180., 96.]),
            [180., 96.],
        )
        .unwrap();
    assert!(v.hit([170., 100.]).is_none());
    assert!(v.hit([1., 1.]).is_none());
}
