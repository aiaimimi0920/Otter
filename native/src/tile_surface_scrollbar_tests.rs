//! 手写像素/几何oracle核四向旋转、crop和屏外捕获；不靠新映射反算预期。
use super::*;
use serde_json::json;
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}
fn fixture() -> crate::tile_surface_actions::tests::Fixture {
    crate::tile_surface_value::tests::fixture(
        json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},
        "children":[{"id":"content","type":"view","layout":{"height":1020,"minHeight":1020}}]}),
    )
}
fn presentation(f: &crate::tile_surface_actions::tests::Fixture) -> Presentation {
    Presentation::new(
        crate::surface_scene::Scene::parse(&f.art.snapshot.scene).unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context([64., 64.]),
        [64., 64.],
    )
    .unwrap()
}
fn frame(f: &crate::tile_surface_actions::tests::Fixture, rotation: &str) -> Frame {
    let mut layout = f.frame.layout.clone();
    layout.tiles[0].rotation = serde_json::from_value(json!(rotation)).unwrap();
    Frame {
        key: f.frame.key.clone(),
        layout,
        size: crate::tile_wire::Size {
            width: 64,
            height: 64,
        },
        images: Default::default(),
        arts: f.frame.arts.clone(),
        live_until: None,
    }
}
#[test]
fn captured_scrollbar_coordinates_obey_four_rotations_and_allow_outside_only_after_hit() {
    let _com = Apartment::new();
    let f = fixture();
    let p = presentation(&f);
    for (rotation, hit, outside) in [
        ("deg0", [60, 5], [60, 95]),
        ("deg90", [5, 3], [95, 3]),
        ("deg180", [3, 58], [3, -32]),
        ("deg270", [58, 60], [-32, 60]),
    ] {
        let frame = frame(&f, rotation);
        let field = Field::at(&frame, &f.art, &p, hit).unwrap();
        assert_eq!(field.point, [60.5, 5.5], "{rotation}");
        assert_eq!(
            field.captured_point(&frame, outside),
            Some([60.5, 95.5]),
            "{rotation}"
        );
        assert!(
            Field::at(&frame, &f.art, &p, outside).is_none(),
            "{rotation}"
        );
    }
}
#[test]
fn crop_nonuniform_output_and_top_placement_share_the_original_hit_contract() {
    let _com = Apartment::new();
    let f = fixture();
    let p = presentation(&f);
    let mut frame = frame(&f, "deg0");
    frame.size = crate::tile_wire::Size {
        width: 128,
        height: 96,
    };
    frame.layout.placements[0].source_crop.x = 0.5;
    frame.layout.placements[0].source_crop.width = 0.5;
    let field = Field::at(&frame, &f.art, &p, [112, 8]).unwrap();
    assert!((field.point[0] - 60.125).abs() < 0.001 && (field.point[1] - 5.666667).abs() < 0.001);
    let outside = field.captured_point(&frame, [112, 101]).unwrap();
    assert!((outside[0] - 60.125).abs() < 0.001 && (outside[1] - 67.666667).abs() < 0.001);
    let mut cover = frame.layout.placements[0].clone();
    cover.placement_id = "cover".into();
    cover.z_index = 2;
    cover.interactive = false;
    frame.layout.placements.push(cover);
    assert!(Field::at(&frame, &f.art, &p, [112, 8]).is_none());
}
#[test]
fn identity_ignores_only_thumb_motion_and_rejects_changed_layout_view_generation_and_output() {
    let _com = Apartment::new();
    let f = fixture();
    let mut p = presentation(&f);
    let frame = frame(&f, "deg0");
    let field = Field::at(&frame, &f.art, &p, [60, 5]).unwrap();
    let old = p.scroll_state();
    let next = p.planned_scrollbar("root", 1, 64.).unwrap();
    assert!(
        p.scroll_state() == old,
        "planning does not mutate Presentation"
    );
    p.restore_scroll(&next).unwrap();
    assert!(field.refreshed(&frame, &f.art, &p).is_some());
    let mut changed = field.clone();
    changed.generation += 1;
    assert!(!field.same(&changed));
    changed = field.clone();
    changed.view.attachment_id = "other".into();
    assert!(!field.same(&changed));
    changed = field.clone();
    changed.output.width += 1;
    assert!(!field.same(&changed));
    changed = field.clone();
    changed.layout[0] ^= 1;
    assert!(!field.same(&changed));
    assert!(p.planned_scrollbar("root", 2, 10.).is_none());
    assert!(p.planned_scrollbar("root", 1, f32::NAN).is_none());
}
