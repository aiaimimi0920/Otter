//! 独立几何/命中oracle；不把find自身产生的期望再喂回被测搜索。
use super::*;
use crate::tile_surface_value::tests::{fixture, presentation};
use serde_json::json;
fn frame(f: &crate::tile_surface_actions::tests::Fixture) -> Frame {
    let mut frame = Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: crate::tile_wire::Size {
            width: 64,
            height: 64,
        },
        arts: f.frame.arts.clone(),
        images: f.frame.images.clone(),
        live_until: None,
    };
    let unit = crate::tile_wire::Rect {
        x: 0.,
        y: 0.,
        width: 1.,
        height: 1.,
    };
    frame.layout.bounds = unit;
    frame.layout.tiles[0].rect = unit;
    frame.layout.placements[0].rect = unit;
    frame
}
#[test]
fn crop_clips_editor_and_focus_before_rotation_and_never_leaks_outside_placement() {
    let f = fixture(json!({"id":"field","type":"input","layout":{"width":64,"height":64}}));
    let p = presentation(&f);
    let mut frame = frame(&f);
    frame.layout.placements[0].rect = crate::tile_wire::Rect {
        x: 0.25,
        y: 0.25,
        width: 0.5,
        height: 0.5,
    };
    frame.layout.placements[0].source_crop = crate::tile_wire::Rect {
        x: 0.25,
        y: 0.25,
        width: 0.5,
        height: 0.5,
    };
    for rotation in [
        crate::tile_wire::Rotation::Zero,
        crate::tile_wire::Rotation::Right,
        crate::tile_wire::Rotation::Half,
        crate::tile_wire::Rotation::Left,
    ] {
        frame.layout.tiles[0].rotation = rotation;
        let focused = find(
            &frame,
            &f.art,
            &p,
            &frame.layout.placements[0],
            "field",
            &mut 8192,
        )
        .unwrap()
        .unwrap();
        assert_eq!(focused.rect, [16, 16, 48, 48]);
        let text = crate::tile_surface_text::hit(&frame, &f.art, &p, focused.anchor.pixel).unwrap();
        assert_eq!(text.rect, [16, 16, 48, 48]);
    }
}
#[test]
fn higher_image_blocks_but_tiny_exposed_strip_is_found_and_budget_is_explicit() {
    let f = fixture(
        json!({"id":"button","type":"button","layout":{"width":64,"height":64},"events":{"click":"go"}}),
    );
    let p = presentation(&f);
    let mut frame = frame(&f);
    let mut cover = frame.layout.placements[0].clone();
    cover.placement_id = "cover".into();
    cover.z_index = 9;
    cover.interactive = false;
    cover.source.kind = "image".into();
    frame.layout.placements.push(cover);
    assert!(find(
        &frame,
        &f.art,
        &p,
        &frame.layout.placements[0],
        "button",
        &mut 1
    )
    .unwrap()
    .is_none());
    frame.layout.placements[1].rect.width = 63. / 64.;
    let found = find(
        &frame,
        &f.art,
        &p,
        &frame.layout.placements[0],
        "button",
        &mut 8192,
    )
    .unwrap()
    .unwrap();
    assert_eq!(found.anchor.pixel[0], 63);
    assert_eq!(
        find(
            &frame,
            &f.art,
            &p,
            &frame.layout.placements[0],
            "button",
            &mut 0
        )
        .err(),
        Some("Art键盘可见区域搜索预算不足")
    );
}
#[test]
fn disabled_hidden_and_noninteractive_never_become_focus() {
    for (disabled, visible, interactive) in [
        (true, true, true),
        (false, false, true),
        (false, true, false),
    ] {
        let f = fixture(
            json!({"id":"button","type":"button","props":{"disabled":disabled,"visible":visible},"layout":{"width":64,"height":64}}),
        );
        let mut frame = frame(&f);
        frame.layout.placements[0].interactive = interactive;
        assert!(find(
            &frame,
            &f.art,
            &presentation(&f),
            &frame.layout.placements[0],
            "button",
            &mut 8192
        )
        .unwrap()
        .is_none());
    }
}
#[test]
fn rounded_overlay_corners_are_not_discarded_as_solid_rectangles() {
    let f = fixture(
        json!({"id":"root","type":"stack","layout":{"width":64,"height":64},"children":[
        {"id":"back","type":"button","layout":{"width":64,"height":64},"style":{"borderRadius":0}},
        {"id":"circle","type":"view","layout":{"width":64,"height":64},"style":{"borderRadius":32,"opacity":0}}]}),
    );
    let p = presentation(&f);
    let frame = frame(&f);
    assert_eq!(p.hit_test([0.5, 0.5]), Some("back"));
    assert_eq!(p.hit_test([32., 32.]), Some("circle"));
    let focus = find(
        &frame,
        &f.art,
        &p,
        &frame.layout.placements[0],
        "back",
        &mut 8192,
    )
    .unwrap()
    .unwrap();
    let [x, y] = focus.anchor.pixel;
    assert_eq!(p.hit_test([x as f32 + 0.5, y as f32 + 0.5]), Some("back"));
}
