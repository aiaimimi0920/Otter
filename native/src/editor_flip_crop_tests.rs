//! 对照主干：底图全源翻转，矢量按裁剪框镜像；裁剪视图及框外内容可恢复。
use super::*;
use crate::crop_geometry::Rect;

fn document() -> Editor {
    let mut editor = Editor::with_layers(Layers {
        annotation_size: None,
        size: (16, 8),
        base: (0..128)
            .flat_map(|i| [i as u8, 0, 0, 255])
            .collect::<Vec<_>>()
            .into(),
        annotations: None,
    })
    .unwrap();
    editor.begin(Tool::Line, Point { x: 3.0, y: 2.0 }).unwrap();
    editor.extend(Point { x: 7.0, y: 4.0 }).unwrap();
    assert!(editor.commit());
    assert!(editor
        .crop_to(Rect {
            x: 2.0,
            y: 1.0,
            width: 8.0,
            height: 4.0
        })
        .unwrap());
    editor
}
fn flip(editor: &mut Editor, axis: Axis) {
    let plan = editor
        .prepare_flip(axis, |_| unreachable!())
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    assert!(editor.commit_raster_refresh(plan).unwrap());
}
#[test]
fn cropped_overlay_flip_resizes_texture_and_atomically_restores_original_dimensions() {
    let source = Layers {
        annotation_size: Some((2, 1)),
        size: (16, 8),
        base: (0..128)
            .flat_map(|i| [i as u8, 0, 0, 255])
            .collect::<Vec<_>>()
            .into(),
        annotations: Some(vec![0, 0, 255, 255, 0, 255, 0, 255].into()),
    };
    let mut editor = Editor::with_layers(source.clone()).unwrap();
    editor.begin(Tool::Line, Point { x: 5.0, y: 3.0 }).unwrap();
    editor.extend(Point { x: 7.0, y: 4.0 }).unwrap();
    editor.commit();
    editor
        .begin_blur(
            Point { x: 6.0, y: 4.0 },
            crate::blur_image::BlurImage::with_strength(16, 8, vec![0; 512].into(), 2.4).unwrap(),
        )
        .unwrap();
    editor.commit();
    let old_blur = editor.elements()[1].blur.as_ref().unwrap().clone();
    editor
        .crop_to(Rect {
            x: 4.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let crop = editor.crop_view();
    let revision = editor.revision();
    flip(&mut editor, Axis::Horizontal);
    assert_eq!(editor.revision(), revision + 1);
    assert_eq!(editor.crop_view(), crop);
    let current = editor.layers().unwrap();
    assert_eq!(current.annotation_dimensions(), (4, 4));
    assert_eq!(
        &current.annotations.as_ref().unwrap()[..4],
        &[0, 255, 0, 255]
    );
    assert_eq!(
        &current.annotations.as_ref().unwrap()[12..16],
        &[0, 0, 255, 255]
    );
    assert_eq!(current.base[0], 15);
    assert_eq!(editor.elements()[0].points[0], Point { x: 7.0, y: 3.0 });
    let composed =
        crate::raster_source::compose_in_frame(current, Some(crop.unwrap().source.rect), || false)
            .unwrap();
    let expected = crate::blur::region_precise(
        &composed,
        (16, 8),
        crate::blur::Region {
            x: 0,
            y: 0,
            width: 16,
            height: 8,
        },
        2.4,
        || false,
    )
    .unwrap();
    assert_eq!(
        &editor.elements()[1].blur.as_ref().unwrap().pixels()[..],
        &expected[..]
    );
    for _ in 0..3 {
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&source));
        assert_eq!(editor.layers().unwrap().annotation_dimensions(), (2, 1));
        assert!(editor.elements()[1]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old_blur.pixels()));
        assert!(editor.redo());
        assert_eq!(editor.layers().unwrap().annotation_dimensions(), (4, 4));
    }
}
#[test]
fn fractional_crop_flip_rounds_layer_dimensions_and_uncropped_flip_refits_full_frame() {
    let mut editor = document();
    let old = editor.layers().unwrap().clone();
    let mut source = old.clone();
    source.annotations = Some([20, 40, 60, 128].repeat(128).into());
    editor
        .replace_layers(editor.revision(), &old, source)
        .unwrap();
    editor
        .crop_to(Rect {
            x: 0.25,
            y: 0.25,
            width: 5.6,
            height: 2.4,
        })
        .unwrap();
    flip(&mut editor, Axis::Vertical);
    assert_eq!(editor.layers().unwrap().annotation_dimensions(), (6, 2));
    assert!(editor
        .layers()
        .unwrap()
        .annotations
        .as_ref()
        .unwrap()
        .chunks_exact(4)
        .all(|p| p == [20, 40, 60, 128]));
    assert!(editor.reset_crop());
    flip(&mut editor, Axis::Horizontal);
    assert_eq!(editor.layers().unwrap().annotation_dimensions(), (16, 8));
    assert!(editor.undo());
    assert_eq!(editor.layers().unwrap().annotation_dimensions(), (6, 2));
}

#[test]
fn offset_crop_mirrors_geometry_locally_but_pixels_across_full_source() {
    let mut editor = document();
    let source = editor.layers().unwrap().clone();
    let crop = editor.crop_view();
    let revision = editor.revision();
    flip(&mut editor, Axis::Horizontal);
    assert_eq!(editor.revision(), revision + 1);
    assert_eq!(editor.crop_view(), crop);
    assert_eq!(
        editor.elements()[0].points,
        [Point { x: 9.0, y: 2.0 }, Point { x: 5.0, y: 4.0 }]
    );
    // 可见左上角 (2,1) 取全源 (13,1)，不是可见片段右端 (9,1)。
    assert_eq!(editor.layers().unwrap().base[(16 + 2) * 4], 29);
    for _ in 0..3 {
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&source));
        assert_eq!(editor.crop_view(), crop);
        assert_eq!(editor.elements()[0].points[0], Point { x: 3.0, y: 2.0 });
        assert!(editor.redo());
        assert_eq!(editor.elements()[0].points[0], Point { x: 9.0, y: 2.0 });
    }
    assert!(editor.reset_crop());
    assert_eq!(editor.layers().unwrap().base[0], 15);
    assert_eq!(editor.elements()[0].points[0].x, 9.0);
    assert!(editor.undo());
    assert_eq!(editor.crop_view(), crop);
}
#[test]
fn repeated_crop_vertical_flip_preserves_outside_geometry_and_double_flip() {
    let mut editor = document();
    assert!(editor
        .crop_to(Rect {
            x: 1.0,
            y: 2.0,
            width: 4.0,
            height: 2.0
        })
        .unwrap());
    let crop = editor.crop_view();
    let source = editor.layers().unwrap().clone();
    let points = editor.elements()[0].points.clone();
    flip(&mut editor, Axis::Vertical);
    assert_eq!(editor.elements()[0].points[0], Point { x: 3.0, y: 6.0 });
    assert_eq!(editor.elements()[0].points[1], Point { x: 7.0, y: 4.0 });
    assert_eq!(editor.crop_view(), crop);
    flip(&mut editor, Axis::Vertical);
    assert_eq!(editor.elements()[0].points, points);
    assert_eq!(&editor.layers().unwrap().base[..], &source.base[..]);
}
#[test]
fn crop_change_while_computing_rejects_candidate_without_consuming_redo() {
    let mut editor = document();
    let plan = editor
        .prepare_flip(Axis::Horizontal, |_| unreachable!())
        .unwrap()
        .unwrap();
    assert!(editor.reset_crop());
    assert!(editor.undo());
    assert!(!editor
        .commit_raster_refresh(plan.compute(|| false).unwrap())
        .unwrap());
    assert!(editor.redo());
    assert!(editor.crop_view().is_none());
}
