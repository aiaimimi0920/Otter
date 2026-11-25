//! Candidate encoding must pass actual restoration and composed readback, not merely JSON-field comparison.
use super::*;
use crate::desktop::{
    image::Image,
    session_candidate::{self, VectorCandidate},
};
use crate::session_source::Source;
fn url(image: &Image, cancel: &AtomicBool) -> String {
    let png = crate::desktop::export::png_raw(image, cancel).unwrap();
    crate::loom_base64::encode_png_data_url(&png, cancel)
        .unwrap()
        .as_str()
        .to_owned()
}

#[test]
fn cropped_source_overlay_and_vectors_keep_composed_pixels_after_save_restore() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let base = Image {
        width: 8,
        height: 4,
        pixels: (0u8..32)
            .flat_map(|index| [index * 5, index * 3, index * 7, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let overlay = Image {
        width: 2,
        height: 2,
        pixels: vec![0, 0, 128, 128, 0, 128, 0, 128, 128, 0, 0, 128, 0, 0, 0, 0].into(),
    };
    let base_url = url(&base, &cancel);
    let overlay_url = url(&overlay, &cancel);
    for flip in [false, true] {
        let source = serde_json::to_vec(&serde_json::json!({"stickers":[{
            "id":"one","x":10,"y":20,"w":4,"h":2,"src":base_url,"rasterizedAnnotationLayerSrc":overlay_url,
            "imageEditState":{"cropRect":{"x":2,"y":1,"w":4,"h":2},"sourceSize":{"w":8,"h":4},"flippedX":flip},
            "annotationState":{"serialCounter":0,"elements":[{"id":"line","type":"line","zIndex":0,"points":[{"x":1,"y":1},{"x":3,"y":1}],"style":{"width":1,"color":"#fff"}}]}
        }],"links":[]})).unwrap();
        let first = VectorCandidate::prepare(
            session_candidate::prepare(source.clone(), None, &cancel).unwrap(),
            &cancel,
        )
        .unwrap();
        let editor = &first.vectors[0].1.editor;
        let source_image = first.candidate.images[0].source.as_ref().unwrap();
        let before =
            crate::desktop::display_composition::compose(source_image, editor, &cancel).unwrap();
        let mut doc = super::tests::doc("one");
        doc.editor = editor.export_snapshot();
        doc.frame = crate::pin_geometry::PinRect {
            x: 10,
            y: 20,
            width: 4,
            height: 2,
        };
        let frame = doc.editor.visible_frame().unwrap();
        doc.viewport = crate::viewport::Viewport {
            scale: 1.0,
            origin: crate::viewport::Point {
                x: -frame.x as f32,
                y: -frame.y as f32,
            },
        };
        doc.restored_source = Some(source_image.clone());
        let mut data = super::tests::data(vec![doc]);
        data.original = Source::from(source);
        let saved = data.encode(&cancel).unwrap();
        let second = VectorCandidate::prepare(
            session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap(),
            &cancel,
        )
        .unwrap();
        let restored = &second.vectors[0].1.editor;
        let after = crate::desktop::display_composition::compose(
            second.candidate.images[0].source.as_ref().unwrap(),
            restored,
            &cancel,
        )
        .unwrap();
        assert_eq!((before.width, before.height), (4, 2));
        assert_eq!((after.width, after.height), (4, 2));
        assert_eq!(before.pixels, after.pixels, "flip={flip}");
        assert_eq!(
            editor.layers().unwrap().annotations,
            restored.layers().unwrap().annotations
        );
        assert_eq!(editor.elements()[0].points, restored.elements()[0].points);
        assert_eq!(
            editor.elements()[0].annotation_id(),
            restored.elements()[0].annotation_id()
        );
    }
}

#[test]
fn long_cropped_session_restores_natural_source_and_editable_layers_at_scaled_dpi() {
    use crate::{
        crop_geometry::Rect,
        editor::{Editor, Tool},
        erase_layers::Layers,
        viewport::{Point, Viewport},
    };

    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    for vertical in [false, true] {
        let (width, height) = if vertical { (48, 16384) } else { (16384, 48) };
        let image = Image {
            width,
            height,
            pixels: (0..width * height)
                .flat_map(|i| {
                    let (x, y) = (i % width, i / width);
                    [x as u8, y as u8, ((x + y) >> 8) as u8, 255]
                })
                .collect::<Vec<_>>()
                .into(),
        };
        let overlay: crate::pixels::Pixels =
            vec![0, 0, 128, 128, 0, 128, 0, 128, 128, 0, 0, 128, 0, 0, 0, 0].into();
        let mut editor = Editor::with_layers(Layers {
            size: (width, height),
            base: image.pixels.clone(),
            annotation_size: Some((2, 2)),
            annotations: Some(overlay.clone()),
        })
        .unwrap();
        let (x, y, w, h) = if vertical {
            (12., 13000., 24., 256.)
        } else {
            (13000., 12., 256., 24.)
        };
        let point = Point {
            x: x as f32 + 8.,
            y: y as f32 + 8.,
        };
        editor.begin(Tool::Line, point).unwrap();
        editor
            .extend(Point {
                x: point.x + 8.,
                y: point.y + 8.,
            })
            .unwrap();
        assert!(editor.commit());
        let id = editor.elements()[0].annotation_id().to_owned();
        assert!(editor
            .crop_to(Rect {
                x,
                y,
                width: w,
                height: h
            })
            .unwrap());

        // The cropped HWND is small; at 144 DPI this viewport maps the full
        // source to 12288 logical units in a session created at 96 DPI.
        let mut doc = super::tests::doc("long");
        doc.editor = editor.export_snapshot();
        doc.frame.width = (w * 0.75) as u32;
        doc.frame.height = (h * 0.75) as u32;
        doc.dpi = 144;
        doc.viewport = Viewport {
            scale: 0.5,
            origin: Point {
                x: -0.5 * x as f32,
                y: -0.5 * y as f32,
            },
        };
        doc.restored_source = Some(image.clone());
        let saved = super::tests::data(vec![doc]).encode(&cancel).unwrap();
        let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
        let geometry = candidate.placements[0].crop.unwrap().editor_crop().unwrap();
        assert_eq!(
            geometry.rect,
            Rect {
                x: x * 0.75,
                y: y * 0.75,
                width: w * 0.75,
                height: h * 0.75
            }
        );
        let vectors = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let prepared = crate::desktop::session_binding::Bound::prepare_for(vectors, None, &cancel)
            .ok()
            .unwrap()
            .into_parts_for(None, &cancel)
            .ok()
            .unwrap()
            .prepare_states(&cancel)
            .unwrap();
        let mut canvas = prepared.states[0].canvas.borrow_mut();
        let source = canvas.minified_source.as_ref().unwrap().image();
        assert_eq!((source.width, source.height), (width, height));
        assert_eq!(source.pixels, image.pixels);
        let editor = &mut canvas.editor;
        let crop = editor.crop_view();
        let layers = editor.layers().unwrap().clone();
        assert_eq!(layers.size, (width * 3 / 4, height * 3 / 4));
        assert_eq!(layers.annotation_dimensions(), (2, 2));
        assert_eq!(layers.annotations.as_ref(), Some(&overlay));
        assert_eq!(editor.elements()[0].annotation_id(), id);
        assert_eq!(
            editor.elements()[0].points[0],
            Point {
                x: point.x * 0.75,
                y: point.y * 0.75
            }
        );
        assert!(!editor.undo());
        assert!(editor.reset_crop());
        assert_eq!(
            editor.visible_frame(),
            Some(Rect {
                x: 0.,
                y: 0.,
                width: width as f64 * 0.75,
                height: height as f64 * 0.75
            })
        );
        assert!(editor.layers().unwrap().same_storage(&layers));
        assert!(editor.undo());
        assert_eq!(editor.crop_view(), crop);
        assert!(editor.redo());
        assert!(editor.crop_view().is_none());
    }
}

#[test]
fn cropped_large_local_pin_reopens_with_pixels_outside_the_crop() {
    large_pin_roundtrip(false);
}

#[test]
fn cropped_large_local_pin_rebuilds_the_blur_from_full_source() {
    large_pin_roundtrip(true);
}

fn large_pin_roundtrip(with_blur: bool) {
    use crate::{
        blur_image::BlurImage,
        crop_geometry::{Rect, Size},
        editor::Editor,
        erase_layers::Layers,
        viewport::Point,
    };
    use windows::Win32::Foundation::{E_ABORT, E_INVALIDARG};

    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    // The source fits the document budget, but exceeds the presentation area limit
    // by one row. Cropping only changes the visible frame in the saved checkpoint.
    let (width, height) = (4096, 4097);
    let mut base = [40, 60, 80, 255].repeat((width * height) as usize);
    base[..4].copy_from_slice(&[10, 20, 30, 255]);
    let overlay: crate::pixels::Pixels = vec![0, 0, 128, 128].into();
    let mut editor = Editor::with_layers(Layers {
        size: (width, height),
        base: base.into(),
        annotation_size: Some((1, 1)),
        annotations: Some(overlay.clone()),
    })
    .unwrap();
    let crop = Rect {
        x: 4000.,
        y: 4000.,
        width: 64.,
        height: 64.,
    };
    assert!(editor.crop_to(crop).unwrap());
    if with_blur {
        // The checkpoint stores the stroke and strength, not this seed texture.
        // Recovery must rebuild the effect from the decoded base and overlay.
        let seed = BlurImage::new(width, height, editor.layers().unwrap().base.clone(), 1).unwrap();
        assert!(editor
            .begin_blur(Point { x: 4032., y: 4032. }, seed)
            .unwrap());
        assert!(editor.commit());
    }
    let mut doc = super::tests::doc("large");
    doc.pinned = true;
    doc.editor = editor.export_snapshot();
    doc.frame.width = 64;
    doc.frame.height = 64;
    doc.viewport.scale = 1.;
    doc.viewport.origin = Point {
        x: -4000.,
        y: -4000.,
    };
    let saved = super::tests::data(vec![doc]).encode(&cancel).unwrap();
    drop(editor);
    let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
    let images = &candidate.images[0];
    let geometry = candidate.placements[0].crop.unwrap().editor_crop().unwrap();
    assert_eq!(
        images
            .prepare_editor_layers(geometry.source, &cancel)
            .err()
            .unwrap()
            .code(),
        E_INVALIDARG
    );
    let mut oversized = geometry;
    oversized.source = Size {
        width: 1_000_001.,
        height: 8193.,
    };
    assert_eq!(
        images
            .prepare_full_crop_layers(oversized, false, &cancel)
            .err()
            .unwrap()
            .code(),
        E_INVALIDARG
    );
    assert_eq!(
        images
            .prepare_full_crop_layers(geometry, false, &AtomicBool::new(true))
            .err()
            .unwrap()
            .code(),
        E_ABORT
    );
    let decoded = images.source.as_ref().unwrap().pixels.clone();
    let decoded_storage = decoded.downgrade();
    let mut vectors = VectorCandidate::prepare(candidate, &cancel).unwrap();
    let editor = &mut vectors.vectors[0].1.editor;
    let layers = editor.layers().unwrap().clone();
    assert_eq!(editor.visible_frame(), Some(crop));
    assert_eq!(layers.size, (width, height));
    assert!(layers.base.same_storage(&decoded));
    assert_eq!(&layers.base[..4], &[10, 20, 30, 255]);
    assert_eq!(layers.annotations.as_ref(), Some(&overlay));
    assert_eq!(layers.annotation_dimensions(), (1, 1));
    let effect_storage = if with_blur {
        let effect = editor.elements()[0].blur.as_ref().unwrap();
        assert_eq!((effect.width(), effect.height()), (width, height));
        let pixel = |x: u32, y: u32| {
            let index = ((y * width + x) * 4) as usize;
            &effect.pixels()[index..index + 4]
        };
        assert_eq!(pixel(10, 10), &[40, 60, 80, 255]);
        assert_eq!(pixel(4032, 4032), &[20, 30, 168, 255]);
        Some(effect.pixels().downgrade())
    } else {
        assert!(editor.elements().is_empty());
        None
    };
    assert!(editor.reset_crop());
    let frame = editor.visible_frame().unwrap();
    assert_eq!(
        (frame.width, frame.height),
        (f64::from(width), f64::from(height))
    );
    assert!(editor.undo());
    assert_eq!(editor.visible_frame(), Some(crop));
    assert!(editor.redo());
    assert_eq!(editor.visible_frame(), Some(frame));
    assert!(editor.layers().unwrap().same_storage(&layers));
    drop(layers);
    drop(decoded);
    drop(vectors);
    assert!(decoded_storage.upgrade().is_none());
    assert!(effect_storage.is_none_or(|storage| storage.upgrade().is_none()));
}

#[path = "application_save_high_zoom_tests.rs"]
mod high_zoom;
