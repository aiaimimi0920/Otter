//! 来源映射与恢复原文一并围栏；相同像素不代表可以复用另一份历史编辑基线。
use super::*;
use crate::{crop_geometry::Size, desktop::canvas::minified_source::Source};

fn image() -> Image {
    Image {
        width: 8,
        height: 4,
        pixels: [20, 30, 40, 255].repeat(32).into(),
    }
}
fn origin() -> Origin {
    Origin {
        unit: crate::workspace_id::UnitId::parse("u").unwrap(),
        record: br#"{"stickers":[{"id":"u","x":0,"y":0,"w":4,"h":4}],"links":[]}"#
            .to_vec()
            .into(),
    }
}

fn legacy_canvas() -> Canvas {
    let working = Image {
        width: 8,
        height: 8,
        pixels: [20, 30, 40, 255].repeat(64).into(),
    };
    let mut canvas = Canvas::new(Some(working)).unwrap();
    let layers = canvas.editor.layers().unwrap().clone();
    canvas.editor = Editor::with_layers(layers)
        .unwrap()
        .with_restored_size(Size {
            width: 8.0,
            height: 8.0,
        })
        .unwrap()
        .with_restored_legacy_crop(
            crate::crop_geometry::Rect {
                x: 4.0,
                y: 0.0,
                width: 4.0,
                height: 8.0,
            },
            Size {
                width: 16.0,
                height: 8.0,
            },
        )
        .unwrap();
    canvas.minified_source = Some(
        Source::new(
            Image {
                width: 16,
                height: 8,
                pixels: [20, 30, 40, 255].repeat(128).into(),
            },
            &canvas.editor,
        )
        .with_origin(origin()),
    );
    canvas
}

#[test]
fn legacy_snapshot_tracks_active_region_and_reset_invalidates_same_pixel_owner() {
    let mut canvas = legacy_canvas();
    let capture = |canvas: &Canvas| {
        Snapshot::capture(
            canvas,
            None,
            crate::projection_delivery::sender::Kind::Sticker,
            &[],
        )
        .unwrap()
    };
    let before = capture(&canvas);
    assert_eq!(
        before.legacy_pixels,
        Some(crate::crop_geometry::Rect {
            x: 4.0,
            y: 0.0,
            width: 4.0,
            height: 8.0,
        })
    );
    let stamp = before.stamp();
    assert!(stamp.matches(&canvas, None, &[]));
    assert!(canvas.editor.reset_crop());
    let reset = capture(&canvas);
    assert!(reset.image.pixels.same_storage(&before.image.pixels));
    assert!(reset.legacy_pixels.is_none());
    assert!(!stamp.matches(&canvas, None, &[]));
    assert!(canvas.editor.undo());
    assert_eq!(capture(&canvas).legacy_pixels, before.legacy_pixels);
    // 来源凭证可再次相同；完整发送生命周期另以DocumentStamp拒绝旧revision。
    assert!(!reset.stamp().matches(&canvas, None, &[]));
}

#[test]
fn edited_legacy_snapshot_uses_working_texture_region_instead_of_retired_natural_pixels() {
    let mut canvas = legacy_canvas();
    let natural = Snapshot::capture(
        &canvas,
        None,
        crate::projection_delivery::sender::Kind::Sticker,
        &[],
    )
    .unwrap();
    let old = canvas.editor.layers().unwrap().clone();
    let mut replacement = old.clone();
    replacement.base = [90, 80, 70, 255].repeat(64).into();
    assert!(canvas
        .editor
        .replace_layers(canvas.editor.revision(), &old, replacement)
        .unwrap());
    let edited = Snapshot::capture(
        &canvas,
        None,
        crate::projection_delivery::sender::Kind::Sticker,
        &[],
    )
    .unwrap();
    assert_eq!((edited.image.width, edited.image.height), (8, 8));
    assert_eq!(
        edited.legacy_pixels,
        Some(crate::crop_geometry::Rect {
            x: 2.0,
            y: 2.0,
            width: 2.0,
            height: 4.0,
        })
    );
    assert!(edited.natural_origin.is_none());
    assert!(!edited.pending_content);
    assert_eq!(&edited.image.pixels[..4], &[90, 80, 70, 255]);
    assert!(!natural.stamp().matches(&canvas, None, &[]));
}

#[test]
fn cross_unit_legacy_uses_live_source_view_and_rejects_pre_reset_resolved_frame() {
    let mut upstream = legacy_canvas();
    let unit = crate::workspace_id::UnitId::parse("upstream").unwrap();
    let old = Frame::capture(
        unit.clone(),
        upstream.image.clone().unwrap(),
        Some(&upstream.editor),
    );
    let selected = select_local(&old, &unit, &upstream).unwrap();
    assert_eq!(selected.legacy_pixels.unwrap().x, 4.0);
    assert!(upstream.editor.reset_crop());
    assert!(select_local(&old, &unit, &upstream).is_none());
    let fresh = Frame::capture(
        unit.clone(),
        upstream.image.clone().unwrap(),
        Some(&upstream.editor),
    );
    assert!(select_local(&fresh, &unit, &upstream)
        .unwrap()
        .legacy_pixels
        .is_none());
}
#[test]
fn snapshot_keeps_source_mapping_and_rejects_same_pixels_with_another_origin() {
    let image = image();
    let mut canvas = Canvas::new(Some(image.clone())).unwrap();
    let expected = origin();
    let logical = Size {
        width: 4.0,
        height: 4.0,
    };
    canvas.minified_source = Some(
        Source::new(image.clone(), &canvas.editor)
            .with_source_logical(logical)
            .with_origin(expected.clone()),
    );
    let captured = Snapshot::capture(
        &canvas,
        None,
        crate::projection_delivery::sender::Kind::Sticker,
        &[],
    )
    .unwrap();
    assert_eq!(captured.source_logical, Some(logical));
    assert!(captured.natural_origin.as_ref().unwrap().same(&expected));
    let input = captured.render_input();
    assert_eq!(input.source_logical, Some(logical));
    assert!(input
        .baseline
        .as_ref()
        .unwrap()
        .record
        .same_storage(&expected.record));
    assert_eq!(input.baseline.as_ref().unwrap().unit, expected.unit);
    assert!(input.historical_erase && input.content);
    assert_eq!(input.flip, (false, false));
    assert!(!input.legacy_crop);
    let stamp = captured.stamp();
    assert!(stamp.matches(&canvas, None, &[]));
    // 字节相同但分配/恢复身份不同，不得假定它与已经捕获的历史是同一份。
    canvas.minified_source = Some(
        Source::new(image, &canvas.editor)
            .with_source_logical(logical)
            .with_origin(origin()),
    );
    assert!(!stamp.matches(&canvas, None, &[]));
}
#[test]
fn materialized_image_has_no_inferred_natural_source_mapping_or_history() {
    let canvas = Canvas::new(Some(image())).unwrap();
    let captured = Snapshot::capture(
        &canvas,
        None,
        crate::projection_delivery::sender::Kind::Sticker,
        &[],
    )
    .unwrap();
    assert!(captured.source_logical.is_none());
    assert!(captured.natural_origin.is_none());
    let input = captured.render_input();
    assert!(input.baseline.is_none());
    assert!(!input.historical_erase && !input.content && !input.legacy_crop);
    assert_eq!(input.flip, (false, false));
    assert!(captured.stamp().matches(&canvas, None, &[]));
}

#[test]
fn cross_unit_mapping_keeps_original_density_and_target_crop_space_separate() {
    let mut upstream = Canvas::new(Some(image())).unwrap();
    let baseline = origin();
    let source_logical = Size {
        width: 4.0,
        height: 4.0,
    };
    upstream.minified_source = Some(
        Source::new(image(), &upstream.editor)
            .with_source_logical(source_logical)
            .with_origin(baseline.clone()),
    );
    let frame = Frame::capture(
        baseline.unit.clone(),
        upstream.image.clone().unwrap(),
        Some(&upstream.editor),
    );
    let mut target = Canvas::new(Some(Image {
        width: 16,
        height: 8,
        pixels: [0, 0, 0, 255].repeat(128).into(),
    }))
    .unwrap();
    target
        .editor
        .crop_to(crate::crop_geometry::Rect {
            x: 4.0,
            y: 2.0,
            width: 8.0,
            height: 4.0,
        })
        .unwrap();
    assert_ne!(
        upstream.editor.layers().unwrap().size,
        target.editor.layers().unwrap().size
    );
    // 普通Editor未设置恢复logical_size；生产接缝必须从完整底图取框，不能取visible crop。
    assert!(target.editor.full_logical_frame().is_none());
    let selected = select_local(&frame, &baseline.unit, &upstream).unwrap();
    let snapshot = Snapshot {
        image: selected.image,
        editor: target.editor.export_snapshot(),
        opacity: 1.0,
        kind: crate::projection_delivery::sender::Kind::Sticker,
        geometry: None,
        source_logical: selected.logical,
        natural_origin: selected.origin,
        legacy_pixels: selected.legacy_pixels,
        pending_content: selected.content,
    };
    let input = snapshot.render_input();
    assert_eq!(input.source_logical, Some(source_logical));
    assert_eq!(
        input.logical,
        Some(Size {
            width: 16.0,
            height: 8.0
        })
    );
    assert_eq!(snapshot.editor.crop_view().unwrap().source.rect.width, 8.0);
    assert!(input
        .baseline
        .unwrap()
        .record
        .same_storage(&baseline.record));
    // 目标reset不改来源密度；来源自身修改则旧resolved立即失效。
    assert!(target.editor.reset_crop());
    assert_eq!(
        select_local(&frame, &baseline.unit, &upstream)
            .unwrap()
            .logical,
        Some(source_logical)
    );
    upstream
        .editor
        .crop_to(crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 2.0,
        })
        .unwrap();
    assert!(select_local(&frame, &baseline.unit, &upstream).is_none());
}
