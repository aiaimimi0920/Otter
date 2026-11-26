//! Transactional raster refresh and lossless multi-step undo/redo regressions.
use super::*;

fn document() -> Editor {
    let mut editor = Editor::with_layers(Layers {
        annotation_size: None,
        size: (16, 16),
        base: [0, 0, 255, 255].repeat(256).into(),
        annotations: None,
    })
    .unwrap();
    for strength in [2.4, 2.4, 4.0] {
        let image = BlurImage::with_strength(16, 16, [0, 0, 255, 255].repeat(256).into(), strength)
            .unwrap();
        editor.begin_blur(Point { x: 8.0, y: 8.0 }, image).unwrap();
        assert!(editor.commit());
    }
    editor
}

fn candidate(editor: &Editor) -> Layers {
    let mut layers = editor.layers().unwrap().clone();
    layers.annotations = Some([255, 0, 0, 255].repeat(256).into());
    layers
}

#[test]
fn raster_and_all_blurs_commit_and_undo_together_with_shared_strength_results() {
    let mut editor = document();
    let original = editor.layers().unwrap().clone();
    let old = editor.elements()[0].blur.clone().unwrap();
    let points = editor.elements()[0].points.as_ptr();
    let revision = editor.revision();
    let next = candidate(&editor);
    let plan = editor
        .prepare_raster_refresh(next.clone())
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    assert_eq!(editor.revision(), revision);
    assert!(editor.commit_raster_refresh(plan).unwrap());
    assert_eq!(editor.revision(), revision + 1);
    let blur = editor.elements()[0].blur.as_ref().unwrap();
    assert_eq!(blur.strength(), 2.4);
    assert!(blur
        .pixels()
        .same_storage(editor.elements()[1].blur.as_ref().unwrap().pixels()));
    let expected = blur::region_precise(
        next.annotations.as_ref().unwrap(),
        next.size,
        blur::Region {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        },
        2.4,
        || false,
    )
    .unwrap();
    assert_eq!(&blur.pixels()[..], &expected[..]);
    for _ in 0..3 {
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&original));
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old.pixels()));
        assert_eq!(editor.elements()[0].points.as_ptr(), points);
        assert!(editor.redo());
        assert!(editor.layers().unwrap().same_storage(&next));
    }
}

#[test]
fn many_localized_raster_refreshes_round_trip_exact_pixels_and_frame() {
    let size = (64u32, 64u32);
    let mut base_vec = [40u8, 80, 160, 255].repeat(64 * 64);
    base_vec[0..4].copy_from_slice(&[10, 20, 30, 255]);
    let base_pixels: crate::pixels::Pixels = base_vec.into();
    let frame = crate::crop_geometry::Rect {
        x: 3.0,
        y: 5.0,
        width: 48.0,
        height: 48.0,
    };
    let initial_blur = BlurImage::prepare(
        &base_pixels,
        size,
        blur::Region {
            x: 0,
            y: 0,
            width: 64,
            height: 64,
        },
        1.5,
        None,
        || false,
    )
    .unwrap()
    .with_frame(frame)
    .unwrap();
    let mut editor = Editor::with_layers(Layers {
        annotation_size: None,
        size,
        base: base_pixels,
        annotations: Some(vec![0u8; 64 * 64 * 4].into()),
    })
    .unwrap();
    editor
        .begin_blur(Point { x: 16.0, y: 16.0 }, initial_blur)
        .unwrap();
    assert!(editor.commit());

    let mut expected_states: Vec<(Vec<u8>, Vec<u8>)> = vec![(
        editor
            .layers()
            .unwrap()
            .annotations
            .as_ref()
            .unwrap()
            .to_vec(),
        editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .to_vec(),
    )];
    for step in 0..8usize {
        let mut ann = editor
            .layers()
            .unwrap()
            .annotations
            .as_ref()
            .unwrap()
            .to_vec();
        let px = ((step * 6 + 4) * 64 + (step * 6 + 4)) * 4;
        ann[px..px + 4].copy_from_slice(&[255, (step * 20) as u8, 0, 255]);
        let mut next = editor.layers().unwrap().clone();
        next.annotations = Some(ann.into());
        let plan = editor
            .prepare_raster_refresh(next)
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        let full_source =
            crate::raster_source::compose_in_frames(editor.layers().unwrap(), None, None, || false)
                .unwrap();
        let full_blur = blur::region_precise(
            &full_source,
            size,
            blur::Region {
                x: 0,
                y: 0,
                width: 64,
                height: 64,
            },
            1.5,
            || false,
        )
        .unwrap();
        let actual_blur = editor.elements()[0].blur.as_ref().unwrap().pixels();
        assert_eq!(&actual_blur[..], &full_blur[..]);
        expected_states.push((
            editor
                .layers()
                .unwrap()
                .annotations
                .as_ref()
                .unwrap()
                .to_vec(),
            actual_blur.to_vec(),
        ));
    }

    for step in (0..8).rev() {
        assert!(editor.undo());
        assert_eq!(
            editor
                .layers()
                .unwrap()
                .annotations
                .as_ref()
                .unwrap()
                .as_ref(),
            expected_states[step].0.as_slice()
        );
        assert_eq!(
            editor.elements()[0]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .as_ref(),
            expected_states[step].1.as_slice()
        );
    }
    assert_eq!(editor.elements()[0].blur.as_ref().unwrap().frame(), frame);
    for step in 1..=8 {
        assert!(editor.redo());
        assert_eq!(
            editor
                .layers()
                .unwrap()
                .annotations
                .as_ref()
                .unwrap()
                .as_ref(),
            expected_states[step].0.as_slice()
        );
        assert_eq!(
            editor.elements()[0]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .as_ref(),
            expected_states[step].1.as_slice()
        );
    }
}

#[test]
fn refresh_recomputes_blurs_from_unrelated_snapshots() {
    let size = (64, 64);
    let mut editor = Editor::with_layers(Layers {
        size,
        annotation_size: None,
        base: vec![255; 64 * 64 * 4].into(),
        annotations: None,
    })
    .unwrap();
    for value in [0, 128] {
        let old = BlurImage::with_strength(64, 64, vec![value; 64 * 64 * 4].into(), 1.5).unwrap();
        assert!(editor.begin_blur(Point { x: 16.0, y: 16.0 }, old).unwrap());
        assert!(editor.commit());
    }
    let mut next = editor.layers().unwrap().clone();
    let mut base = next.base.to_vec();
    base[(32 * 64 + 32) * 4..(32 * 64 + 33) * 4].fill(0);
    next.base = base.into();
    let expected = blur::region_precise(
        &next.base,
        size,
        blur::Region {
            x: 0,
            y: 0,
            width: 64,
            height: 64,
        },
        1.5,
        || false,
    )
    .unwrap();
    let plan = editor
        .prepare_raster_refresh(next)
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    assert!(editor.commit_raster_refresh(plan).unwrap());
    for stroke in editor.elements() {
        assert_eq!(&stroke.blur.as_ref().unwrap().pixels()[..], &expected[..]);
    }
}

#[test]
fn rasterizing_blurs_excludes_removed_effects_and_balances_point_budget() {
    for indices in [vec![0], vec![1], vec![0, 2], vec![0, 1, 2]] {
        let mut editor = document();
        let retained = editor.retained_points;
        let layers = candidate(&editor);
        let plan = editor.prepare_rasterize_edit(&indices).unwrap().unwrap();
        assert_eq!(plan.replacements.len(), 3 - indices.len());
        let extra = plan.extra;
        let plan = plan
            .with_candidate(layers)
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.elements().len(), 3 - indices.len());
        assert_eq!(editor.retained_points, retained + extra);
        assert!(editor.undo());
        assert_eq!(editor.elements().len(), 3);
        editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(editor.commit());
        assert_eq!(editor.retained_points, retained + 1);
    }
}

#[test]
fn pending_edit_with_unchanged_layers_is_noop_and_preserves_redo() {
    let mut editor = document();
    assert!(editor.undo());
    let revision = editor.revision();
    let plan = editor.prepare_raster_edit().unwrap().unwrap();
    assert!(plan.matches_source(editor.layers().unwrap(), revision));
    assert!(!plan.matches_source(editor.layers().unwrap(), revision + 1));
    let plan = plan.compute(|| false).unwrap();
    assert!(!editor.commit_raster_refresh(plan).unwrap());
    assert_eq!(editor.revision(), revision);
    assert!(editor.redo());
}

#[test]
fn cancellation_unprepared_foreign_and_stale_candidates_never_change_document() {
    let mut editor = document();
    let original = editor.layers().unwrap().clone();
    let plan = editor
        .prepare_raster_refresh(candidate(&editor))
        .unwrap()
        .unwrap();
    assert!(!editor.commit_raster_refresh(plan).unwrap());
    let plan = editor
        .prepare_raster_refresh(candidate(&editor))
        .unwrap()
        .unwrap();
    assert!(matches!(
        plan.compute(|| true),
        Err(blur::BlurError::Cancelled)
    ));
    let plan = editor
        .prepare_raster_refresh(candidate(&editor))
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    let mut other = document();
    assert!(!other.commit_raster_refresh(plan).unwrap());
    let plan = editor
        .prepare_raster_refresh(candidate(&editor))
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    editor.undo();
    assert!(!editor.commit_raster_refresh(plan).unwrap());
    assert!(editor.layers().unwrap().same_storage(&original));
    assert!(editor.redo());
}
