//! 真实WIC/D2D嵌入式Art；手工矩阵oracle同时约束crop/旋转/遮挡，不用新renderer算预期。
use super::*;
use crate::{
    tile_surface_wire::{Binding, State},
    tile_wire::*,
};
use serde_json::json;
use std::sync::Arc;
pub(super) fn frame(rotation: Rotation) -> Arc<Frame> {
    let (_, mut frame) = crate::tile_tests::shared_frame();
    frame.layout.tiles[0].rotation = rotation;
    let binding = Binding::from(&frame.key);
    let value = json!({"protocolVersion":"loom.wall.v1","view":{"binding":binding,"instanceId":"art","attachmentId":"view"},"width":2,"height":2,"generation":1,"sequence":0,
        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"art","attachmentId":"view","artId":"test","artVersion":"1","revision":1,"runtime":"declarative","resources":[],"resourceLeases":[],
        "scene":{"id":"root","type":"column","layout":{"width":2,"height":2},"children":[
            {"id":"row1","type":"row","layout":{"width":2,"height":1},"children":[{"id":"a","type":"view","layout":{"width":1,"height":1},"style":{"background":"#010000"}},{"id":"b","type":"view","layout":{"width":1,"height":1},"style":{"background":"#020000"}}]},
            {"id":"row2","type":"row","layout":{"width":2,"height":1},"children":[{"id":"c","type":"view","layout":{"width":1,"height":1},"style":{"background":"#030000"}},{"id":"d","type":"view","layout":{"width":1,"height":1},"style":{"background":"#040000"}}]}]}},"preview":null,"result":null,"confirmations":[],"pending":[],"failure":null});
    let state = State::parse(&serde_json::to_vec(&value).unwrap(), &binding, "art", None).unwrap();
    frame.arts.insert(
        "art".into(),
        Arc::new(crate::tile_surface_owner::Art {
            view: state.view,
            generation: state.generation,
            snapshot: state.snapshot,
            size: state.size,
            images: Default::default(),
        }),
    );
    frame.layout.placements.push(Placement {
        placement_id: "art-p".into(),
        source: Source {
            kind: "surface".into(),
            id: "art".into(),
        },
        rect: Rect {
            x: 0.,
            y: 0.,
            width: 2.,
            height: 2.,
        },
        source_crop: Rect {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.,
        },
        z_index: 0,
        interactive: false,
    });
    Arc::new(frame)
}
#[test]
fn canonical_art_quadrants_rotate_without_an_intermediate_scaled_bitmap() {
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
        .ok()
        .unwrap();
    }
    for (rotation, expected) in [
        (Rotation::Zero, [1, 2, 3, 4]),
        (Rotation::Right, [2, 4, 1, 3]),
        (Rotation::Half, [4, 3, 2, 1]),
        (Rotation::Left, [3, 1, 4, 2]),
    ] {
        let actual = unsafe { tests::render(&frame(rotation)) };
        assert_eq!(
            actual.chunks_exact(4).map(|p| p[2]).collect::<Vec<_>>(),
            expected
        );
    }
    unsafe {
        windows::Win32::System::Com::CoUninitialize();
    }
}
#[test]
fn canonical_crop_clip_and_image_above_art_share_the_same_z_order() {
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
        .ok()
        .unwrap();
    }
    let mut frame = Arc::try_unwrap(frame(Rotation::Zero)).ok().unwrap();
    frame.layout.placements[0].source_crop = Rect {
        x: 0.5,
        y: 0.,
        width: 0.5,
        height: 1.,
    };
    frame.layout.placements[0].rect.width = 1.;
    let frame = Arc::new(frame);
    assert_eq!(
        unsafe { tests::render(&frame) }
            .chunks_exact(4)
            .map(|p| p[2])
            .collect::<Vec<_>>(),
        [2, 0, 4, 0]
    );
    let mut frame = Arc::try_unwrap(frame).ok().unwrap();
    frame.images.insert(
        "upper".into(),
        crate::image_decode::Image {
            width: 1,
            height: 1,
            pixels: vec![0, 128, 0, 128].into(),
        },
    );
    let mut upper = frame.layout.placements[0].clone();
    upper.placement_id = "upper".into();
    upper.source = Source {
        kind: "image".into(),
        id: "upper".into(),
    };
    upper.z_index = 1;
    upper.rect.width = 2.;
    upper.source_crop = Rect {
        x: 0.,
        y: 0.,
        width: 1.,
        height: 1.,
    };
    frame.layout.placements.push(upper);
    let pixels = unsafe { tests::render(&Arc::new(frame)) };
    assert_eq!(pixels[1], 128);
    assert_eq!(pixels[2], 1);
    assert_eq!(pixels[7], 255);
    unsafe {
        windows::Win32::System::Com::CoUninitialize();
    }
}
fn replace_scene(frame: &mut Frame, scene: serde_json::Value, size: [u32; 2]) {
    let art = Arc::get_mut(frame.arts.get_mut("art").unwrap()).unwrap();
    Arc::get_mut(&mut art.snapshot).unwrap().scene = serde_json::to_vec(&scene).unwrap();
    art.size = size;
}
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
#[test]
fn art_above_image_and_second_art_overlap_obey_one_shared_placement_order() {
    let _apartment = Apartment::new();
    let mut f = Arc::try_unwrap(frame(Rotation::Zero)).ok().unwrap();
    let mut lower = f.layout.placements[0].clone();
    lower.source = Source {
        kind: "image".into(),
        id: "lower".into(),
    };
    lower.placement_id = "lower".into();
    lower.z_index = -1;
    f.images.insert(
        "lower".into(),
        crate::image_decode::Image {
            width: 1,
            height: 1,
            pixels: vec![255, 0, 0, 255].into(),
        },
    );
    f.layout.placements.push(lower);
    assert_eq!(
        unsafe { tests::render(&Arc::new(f)) }
            .chunks_exact(4)
            .map(|p| p[2])
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    let mut f = Arc::try_unwrap(frame(Rotation::Zero)).ok().unwrap();
    let mut other = Arc::try_unwrap(frame(Rotation::Zero)).ok().unwrap();
    let empty = crate::surface_presentation::Presentation::new(
        crate::surface_scene::Scene::parse(
            br##"{"id":"green","type":"view","style":{"background":"#00ff00"}}"##,
        )
        .unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context([2., 2.]),
        [2., 2.],
    )
    .unwrap();
    assert_eq!(
        empty.geometry().nodes()[0].size()[1],
        0.,
        "an empty auto-height view is not a painted full viewport"
    );
    replace_scene(
        &mut other,
        json!({"id":"green","type":"view","layout":{"width":2,"height":2},"style":{"background":"#00ff00"}}),
        [2, 2],
    );
    f.arts
        .insert("second".into(), other.arts.remove("art").unwrap());
    let mut p = f.layout.placements[0].clone();
    p.placement_id = "second".into();
    p.source.id = "second".into();
    p.z_index = 1;
    p.rect.x = 1.;
    p.rect.width = 1.;
    f.layout.placements.push(p);
    let pixels = unsafe { tests::render(&Arc::new(f)) };
    assert_eq!(
        pixels.chunks_exact(4).map(|p| p[2]).collect::<Vec<_>>(),
        [1, 0, 3, 0]
    );
    assert_eq!(
        pixels.chunks_exact(4).map(|p| p[1]).collect::<Vec<_>>(),
        [0, 255, 0, 255]
    );
}
#[test]
fn embedded_control_background_is_inside_rounded_parent_and_group_opacity() {
    let _apartment = Apartment::new();
    let mut f = Arc::try_unwrap(frame(Rotation::Zero)).ok().unwrap();
    f.size = Size {
        width: 32,
        height: 32,
    };
    f.layout.tiles[0].rect.width = 32.;
    f.layout.tiles[0].rect.height = 32.;
    f.layout.placements[0].rect.width = 32.;
    f.layout.placements[0].rect.height = 32.;
    replace_scene(
        &mut f,
        json!({"id":"root","type":"view","layout":{"width":32,"height":32},
        "style":{"opacity":0.5,"borderRadius":16,"overflow":"hidden"},"children":[
        {"id":"field","type":"input","layout":{"width":32,"height":32},"style":{"borderRadius":16}}]}),
        [32, 32],
    );
    let pixels = unsafe { tests::render(&Arc::new(f)) };
    for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31)] {
        let i = (y * 32 + x) * 4;
        assert_eq!(&pixels[i..i + 4], &[0, 0, 0, 255]);
    }
    let i = (16 * 32 + 16) * 4;
    for (actual, expected) in pixels[i..i + 3].iter().zip([16u8, 12, 9]) {
        assert!(actual.abs_diff(expected) <= 1);
    }
    assert_eq!(pixels[i + 3], 255);
}
