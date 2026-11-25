//! Real checkpoint and PNG regression for work textures smaller than the logical crop source.
use super::*;

#[test]
fn cropped_pin_high_zoom_checkpoint_keeps_logical_frame_with_bounded_texture() {
    use crate::{
        blur_image::BlurImage,
        crop_geometry::Rect,
        editor::Editor,
        erase_layers::{Layers, Mode},
        viewport::Point,
    };

    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let mut base = [10, 20, 30, 255].repeat(4096 * 8);
    base[..4 * 16].fill(0);
    let overlay: crate::pixels::Pixels = vec![0, 0, 128, 128].into();
    let mut editor = Editor::with_layers(Layers {
        size: (4096, 8),
        base: base.into(),
        annotation_size: Some((1, 1)),
        annotations: Some(overlay.clone()),
    })
    .unwrap();
    assert!(editor
        .crop_to(Rect {
            x: 512.,
            y: 2.,
            width: 64.,
            height: 4.
        })
        .unwrap());
    let seed = BlurImage::new(4096, 8, editor.layers().unwrap().base.clone(), 1).unwrap();
    assert!(editor.begin_blur(Point { x: 544., y: 4. }, seed).unwrap());
    assert!(editor.commit());
    // At 4× the working raster still fits; at 8× only the *visible* window fits.
    for zoom in [4, 8] {
        let mut doc = super::super::tests::doc("zoom");
        doc.pinned = true;
        doc.editor = editor.export_snapshot();
        doc.frame.width = 64 * zoom;
        doc.frame.height = 4 * zoom;
        doc.viewport.scale = zoom as f32;
        doc.viewport.origin = Point {
            x: -512. * zoom as f32,
            y: -2. * zoom as f32,
        };
        let saved = super::super::tests::data(vec![doc])
            .encode(&cancel)
            .unwrap();
        let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
        let decoded = candidate.images[0].source.as_ref().unwrap();
        assert_eq!((decoded.width, decoded.height), (4096, 8));
        let full = candidate.placements[0]
            .crop
            .unwrap()
            .editor_crop()
            .unwrap()
            .source;
        assert_eq!(
            (full.width, full.height),
            (f64::from(4096 * zoom), f64::from(8 * zoom))
        );
        let placement = candidate.placements[0].expanded_rect().unwrap();
        assert_eq!(
            (placement.w, placement.h),
            (f64::from(64 * zoom), f64::from(4 * zoom))
        );
        let mut result = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let restored = &mut result.vectors[0].1.editor;
        let visible = restored.visible_frame().unwrap();
        let logical = restored.full_logical_frame().unwrap();
        let layers = restored.layers().unwrap().clone();
        assert_eq!(layers.size, (16384, 32));
        assert_eq!(&layers.base[..4], &[0; 4]);
        assert_eq!(layers.annotations.as_ref(), Some(&overlay));
        let projection = restored.raster_base_projection();
        assert_eq!(projection.is_some(), zoom == 8);
        let point = restored.elements()[0].points[0];
        assert_eq!(
            point,
            Point {
                x: 544. * zoom as f32,
                y: 4. * zoom as f32
            }
        );
        let effect = restored.elements()[0].blur.as_ref().unwrap();
        assert_eq!(
            effect.frame(),
            if zoom == 8 {
                logical
            } else {
                Rect {
                    width: 16384.,
                    height: 32.,
                    ..logical
                }
            }
        );
        let center = (16 * 16384 + 2176) * 4;
        assert!(effect.pixels()[center + 2] > 128);
        assert_eq!(effect.pixels()[center + 3], 255);
        let image = crate::desktop::image::Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels: layers.base.clone(),
        };
        let (png, width, height) = crate::desktop::export::png_payload(&image, restored).unwrap();
        assert_eq!((width, height), (64 * zoom, 4 * zoom));
        let exported = crate::desktop::export::decode_png(png.to_vec());
        assert_eq!(&exported.pixels[..4], &[5, 10, 143, 255]);
        let candidate = layers
            .erase_in_frames(
                Mode::Content,
                &[point],
                32.,
                projection,
                Some(visible),
                || false,
            )
            .unwrap();
        assert_eq!(candidate.base[center + 3], 0);
        let baseline = effect.pixels().clone();
        let refresh = restored
            .prepare_raster_refresh(candidate)
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(restored.commit_raster_refresh(refresh).unwrap());
        assert!(
            restored.elements()[0].blur.as_ref().unwrap().pixels()[center + 3]
                < baseline[center + 3]
        );
        let reset = restored
            .prepare_crop_refresh(None)
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(restored.commit_raster_refresh(reset).unwrap());
        assert_eq!(restored.visible_frame(), Some(logical));
        assert_eq!(restored.raster_base_projection(), projection);
        assert!(restored.undo());
        assert_eq!(restored.visible_frame(), Some(visible));
        assert!(restored.redo());
        assert_eq!(restored.visible_frame(), Some(logical));
        assert!(restored.undo());

        let mut doc = super::super::tests::doc("zoom");
        doc.pinned = true;
        doc.editor = restored.export_snapshot();
        doc.frame.width = 64 * zoom;
        doc.frame.height = 4 * zoom;
        doc.viewport.scale = 1.0;
        doc.viewport.origin = Point {
            x: -visible.x as f32,
            y: -visible.y as f32,
        };
        let saved = super::super::tests::data(vec![doc])
            .encode(&cancel)
            .unwrap();
        let reopened = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
        let reopened = VectorCandidate::prepare(reopened, &cancel).unwrap();
        let reopened = &reopened.vectors[0].1.editor;
        assert_eq!(reopened.visible_frame(), Some(visible));
        assert_eq!(reopened.layers().unwrap().base[center + 3], 0);
        assert_eq!(reopened.elements()[0].points[0], point);
    }
}

#[test]
fn cropped_pin_fractional_grid_reopens_without_recontaining_materialized_pixels() {
    use crate::{
        crop_geometry::Rect,
        desktop::session_binding::Bound,
        editor::Editor,
        erase_layers::{Layers, Mode},
        viewport::Point,
    };
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let mut editor = Editor::with_layers(Layers {
        size: (4096, 8),
        base: [20, 40, 80, 255].repeat(4096 * 8).into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap();
    let crop = Rect {
        x: 512.,
        y: 2.,
        width: 64.,
        height: 3.75,
    };
    assert!(editor.crop_to(crop).unwrap());
    let save = |editor: &Editor,
                scale: f32,
                retained: Option<crate::desktop::image::Image>,
                rasterized: bool| {
        let mut doc = super::super::tests::doc("fractional");
        doc.pinned = true;
        doc.editor = editor.export_snapshot();
        doc.frame.width = 512;
        doc.frame.height = 33;
        let visible = editor.visible_frame().unwrap();
        doc.viewport.scale = scale;
        doc.viewport.origin = Point {
            x: -visible.x as f32 * scale,
            y: -visible.y as f32 * scale,
        };
        doc.restored_source = retained;
        doc.restored_source_rasterized_crop = rasterized;
        super::super::tests::data(vec![doc])
            .encode(&cancel)
            .unwrap()
    };
    let reopen = |saved: crate::session_write::Candidate| {
        let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
        let marked = candidate.placements[0].crop.unwrap().rasterized_source;
        let vectors = VectorCandidate::prepare(candidate, &cancel).unwrap();
        (marked, vectors)
    };
    let (marked, mut first) = reopen(save(&editor, 8., None, false));
    assert!(!marked, "the natural input must retain contain semantics");
    let restored = &mut first.vectors[0].1.editor;
    let visible = restored.visible_frame().unwrap();
    let source = restored.layers().unwrap().clone();
    assert_eq!(source.size, (16384, 35));
    let projected = restored.raster_base_projection().unwrap();
    assert_ne!(
        projected.target.height / 35.,
        projected.target.width / 16384.
    );
    let edited = source
        .erase_in_frames(
            Mode::Content,
            &[Point { x: 4352., y: 35. }],
            16.,
            Some(projected),
            Some(visible),
            || false,
        )
        .unwrap();
    assert_ne!(edited.base, source.base);
    assert!(restored
        .replace_layers(restored.revision(), &source, edited.clone())
        .unwrap());

    let saved = save(restored, 1., None, false);
    // Reproduce the old read path with the very same edited PNG.
    let mut legacy: serde_json::Value = serde_json::from_slice(&saved.bytes).unwrap();
    legacy["stickers"][0]["imageEditState"]
        .as_object_mut()
        .unwrap()
        .remove(crate::session_crop::RASTERIZED_SOURCE_FIELD)
        .unwrap();
    let legacy =
        session_candidate::prepare(serde_json::to_vec(&legacy).unwrap(), None, &cancel).unwrap();
    let legacy = VectorCandidate::prepare(legacy, &cancel).unwrap();
    assert_ne!(
        legacy.vectors[0].1.editor.layers().unwrap().base,
        edited.base
    );
    let (marked, vectors) = reopen(saved);
    assert!(marked);
    let prepared = Bound::prepare_for(vectors, None, &cancel)
        .ok()
        .unwrap()
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    let canvas = prepared.states[0].canvas.borrow();
    assert_eq!(canvas.editor.visible_frame(), Some(visible));
    assert_eq!(canvas.editor.layers().unwrap().base, edited.base);
    let backing = canvas.minified_source.as_ref().unwrap();
    assert!(backing.matches(&canvas.editor));
    assert!(backing.rasterized_crop());
    assert!(backing
        .image()
        .pixels
        .same_storage(&canvas.editor.layers().unwrap().base));

    for (rasterized, retained) in [(true, Some(backing.image().clone())), (false, None)] {
        let (marked, again) = reopen(save(&canvas.editor, 1., retained, rasterized));
        assert!(marked);
        let again = &again.vectors[0].1.editor;
        assert_eq!(again.visible_frame(), Some(visible));
        assert_eq!(again.layers().unwrap().base, edited.base);
    }
}

#[test]
fn reset_working_grid_checkpoint_reopens_without_crop_or_pixel_drift() {
    use crate::{
        crop_geometry::{Rect, Size},
        desktop::{image::Image, session_binding::Bound},
        editor::Editor,
        erase_layers::Layers,
    };
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let logical = Size {
        width: 128.,
        height: 65.,
    };
    let size = (64, 32);
    let mut row = vec![0; size.0 as usize * 4];
    row[80..160].copy_from_slice(&[20, 40, 80, 255].repeat(20));
    row[120..128].fill(0); // Keep a transparent erase hole inside the marked band.
    let base: crate::pixels::Pixels = row.repeat(size.1 as usize).into();
    let image = Image {
        width: size.0,
        height: size.1,
        pixels: base.clone(),
    };
    let mut editor = Editor::with_layers(Layers {
        size,
        base: base.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_working_size(logical)
    .unwrap();
    assert!(editor
        .crop_to(Rect {
            x: 32.,
            y: 1.,
            width: 64.,
            height: 33.
        })
        .unwrap());
    assert!(editor.reset_crop());
    let save = |editor: &Editor, source: Option<Image>| {
        let mut doc = super::super::tests::doc("reset-grid");
        doc.pinned = true;
        doc.editor = editor.export_snapshot();
        doc.frame.width = 128;
        doc.frame.height = 65;
        doc.viewport.scale = 1.;
        doc.restored_source_rasterized_crop = source.is_some();
        doc.restored_source = source;
        super::super::tests::data(vec![doc])
            .encode(&cancel)
            .unwrap()
    };
    let reopen = |saved: crate::session_write::Candidate| {
        let wire: serde_json::Value = serde_json::from_slice(&saved.bytes).unwrap();
        let edit = &wire["stickers"][0]["imageEditState"];
        assert_eq!(
            edit[crate::session_crop::FULL_RASTERIZED_SOURCE_FIELD],
            true
        );
        for field in [
            "cropRect",
            "sourceSize",
            crate::session_crop::RASTERIZED_SOURCE_FIELD,
        ] {
            assert!(edit.get(field).is_none(), "{field}");
        }
        let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
        assert!(candidate.placements[0].crop.is_none());
        assert!(candidate.placements[0].rasterized_full);
        VectorCandidate::prepare(candidate, &cancel).unwrap()
    };
    let vectors = reopen(save(&editor, Some(image)));
    let png = vectors
        .bake_source_png(
            &crate::workspace_id::UnitId::parse("reset-grid").unwrap(),
            &cancel,
        )
        .unwrap();
    let baked = crate::desktop::export::decode_png(png.to_vec());
    assert_eq!((baked.width, baked.height), size);
    assert_eq!(baked.pixels, base);
    let states = Bound::prepare_for(vectors, None, &cancel)
        .ok()
        .unwrap()
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    let canvas = states.states[0].canvas.borrow();
    let restored = &canvas.editor;
    assert!(restored.crop_view().is_none());
    assert_eq!(restored.visible_frame(), editor.visible_frame());
    let backing = canvas.minified_source.as_ref().unwrap();
    assert!(backing.rasterized_crop() && backing.matches(restored));
    assert!(backing
        .image()
        .pixels
        .same_storage(&restored.layers().unwrap().base));
    assert_eq!(restored.layers().unwrap().base, base);
    for source in [None, Some(backing.image().clone())] {
        let again = reopen(save(restored, source));
        assert_eq!(again.vectors[0].1.editor.layers().unwrap().base, base);
    }
}
