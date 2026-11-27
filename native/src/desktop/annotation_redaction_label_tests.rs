//! Exercise the production vector writer, reader, mouse transactions and WIC flattening.
use super::*;
use crate::{
    editor::{Editor, Tool},
    viewport::Point,
};
fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}
fn reopen(editor: &Editor) -> Editor {
    let bytes = crate::session_vector_write::encode(editor, None, |t| {
        Ok(annotation_text::measure_flip_metrics(t)
            .unwrap()
            .baseline_from_top)
    })
    .unwrap();
    let json: Box<serde_json::value::RawValue> = serde_json::from_slice(&bytes).unwrap();
    crate::session_vectors::restore_with_resources(
        Some(&json),
        |t| {
            Ok(crate::session_vectors::TextMetrics {
                size: unsafe { annotation_text::measure(t).unwrap() },
                baseline: annotation_text::measure_flip_metrics(t)
                    .unwrap()
                    .baseline_from_top,
            })
        },
        |s| {
            Ok(editor
                .elements()
                .iter()
                .filter_map(|e| e.blur.clone())
                .find(|im| im.strength() == s)
                .unwrap())
        },
    )
    .unwrap()
    .editor
}
#[test]
fn overlapping_spotlights_reopen_and_dim_only_once() {
    let _apt = Apartment::new().unwrap();
    let image = image::Image {
        width: 64,
        height: 48,
        pixels: vec![255; 64 * 48 * 4].into(),
    };
    let mut editor = Editor::default();
    assert_eq!(editor.spotlight_percent(), 60);
    assert!(editor.set_spotlight_percent(95).is_err());
    editor.set_spotlight_percent(70).unwrap();
    editor.set_spotlight_percent(70).unwrap();
    assert!(editor.undo());
    assert_eq!(editor.spotlight_percent(), 60);
    assert!(!editor.undo());
    assert!(editor.redo());
    assert_eq!(editor.spotlight_percent(), 70);
    for (tool, x) in [
        (Tool::SpotRect, 8.),
        (Tool::SpotEllipse, 22.),
        (Tool::SpotRounded, 28.),
    ] {
        editor.begin(tool, p(x, 10.)).unwrap();
        editor.extend(p(x + 28., 36.)).unwrap();
        assert!(editor.commit());
    }
    let restored = reopen(&editor);
    assert_eq!(restored.spotlight_percent(), 70);
    let output = export::decode_png(
        export::style::Snapshot::default()
            .encoded_png(&image, &restored, 1.)
            .unwrap()
            .to_vec(),
    );
    let pixel = |x, y| output.pixels[(64 * y + x) * 4];
    for (x, y) in [(16usize, 22usize), (30, 22), (45, 22)] {
        assert_eq!(pixel(x, y), 255);
    }
    assert!((75..=78).contains(&pixel(2, 2)));
    assert!(restored.elements()[2].corner_radius > 0.);
}
#[test]
fn magnifier_reopens_and_samples_only_redacted_pixels_across_both_edges() {
    let _apt = Apartment::new().unwrap();
    let image = image::Image {
        width: 64,
        height: 48,
        pixels: (0..64 * 48)
            .flat_map(|i| {
                if i % 2 == 0 {
                    [255u8, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect::<Vec<_>>()
            .into(),
    };
    let render = |editor: &Editor| {
        export::decode_png(
            export::style::Snapshot::default()
                .encoded_png(&image, editor, 1.)
                .unwrap()
                .to_vec(),
        )
    };
    let pixel = |img: &image::Image, x: usize, y: usize| {
        <[u8; 4]>::try_from(&img.pixels[4 * (y * 64 + x)..4 * (y * 64 + x) + 4]).unwrap()
    };
    for tool in [Tool::Mosaic, Tool::Blur] {
        let mut editor = Editor::default();
        if tool == Tool::Blur {
            editor
                .begin_blur(
                    p(8., 5.),
                    crate::blur_image::BlurImage::new(
                        64,
                        48,
                        [125u8, 123, 129, 255].repeat(64 * 48).into(),
                        8,
                    )
                    .unwrap(),
                )
                .unwrap();
        } else {
            editor.begin(tool, p(8., 5.)).unwrap();
        }
        editor.mark_effect_rect().unwrap();
        editor.extend(p(25., 40.)).unwrap();
        assert!(editor.commit());
        let expected = render(&editor);
        editor.begin(Tool::SpotRect, p(1., 1.)).unwrap();
        editor.extend(p(3., 3.)).unwrap();
        assert!(editor.commit());
        editor.begin(Tool::Magnifier, p(41., 14.)).unwrap();
        editor.extend(p(58., 31.)).unwrap();
        assert!(editor.commit());
        for x in [7, 8, 9, 24, 25, 26] {
            let zoom = if x % 2 == 0 { 251 } else { 250 };
            editor.set_lens_zoom(2, zoom).unwrap();
            editor
                .move_lens_handle(2, 0, p(x as f32 + 0.5, 22.5))
                .unwrap();
            let restored = reopen(&editor);
            assert_eq!(restored.elements()[2].lens.unwrap().percent, zoom);
            let output = render(&restored);
            let sample = pixel(&output, 49, 22);
            let safe = pixel(&expected, x, 22);
            assert!(
                sample.iter().zip(safe).all(|(a, b)| a.abs_diff(b) <= 2),
                "{tool:?} at {x}: {sample:?} != {safe:?}"
            );
        }
    }
}
#[test]
fn magnifier_budget_denial_prevents_final_bitmap_publication() {
    use windows::Win32::{Graphics::Imaging::*, System::Com::*};
    let _apt = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let image = image::Image {
        width: 32,
        height: 32,
        pixels: vec![255u8; 32 * 32 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.begin(Tool::Magnifier, p(3., 3.)).unwrap();
    editor.extend(p(18., 18.)).unwrap();
    assert!(editor.commit());
    let calls = std::cell::Cell::new(0);
    let output = export::compose_with_budget(&image, &editor, &wic, |n| {
        calls.set(calls.get() + 1);
        if calls.get() <= 2 {
            crate::pixels::reserve(n)
        } else {
            None
        }
    });
    assert!(matches!(output,Err(e) if e.code()==windows::Win32::Foundation::E_OUTOFMEMORY));
    assert_eq!(calls.get(), 3);
}
#[test]
fn rectangle_redaction_reopens_as_a_solid_area_not_as_a_diagonal_path() {
    let _apt = Apartment::new().unwrap();
    let image = image::Image {
        width: 48,
        height: 48,
        pixels: [29u8, 57, 111, 255].repeat(48 * 48).into(),
    };
    for tool in [Tool::Mosaic, Tool::Blur] {
        let mut e = Editor::default();
        if tool == Tool::Blur {
            let blur = crate::blur_image::BlurImage::new(
                48,
                48,
                [204u8, 192, 180, 255].repeat(48 * 48).into(),
                8,
            )
            .unwrap();
            e.begin_blur(p(40., 40.), blur).unwrap();
        } else {
            e.begin(tool, p(40., 40.)).unwrap();
        }
        e.mark_effect_rect().unwrap();
        e.extend(p(4., 4.)).unwrap();
        assert!(e.commit());
        let restored = reopen(&e);
        assert_eq!(restored.elements()[0].tool, tool);
        assert!(restored.elements()[0].effect_rect);
        let bytes = export::style::Snapshot::default()
            .encoded_png(&image, &restored, 1.)
            .unwrap();
        let output = export::decode_png(bytes.to_vec());
        for (x, y) in [(5, 5), (38, 5), (21, 21), (5, 38), (38, 38)] {
            let i = (48 * y + x) * 4;
            assert_ne!(&output.pixels[i..i + 4], &image.pixels[i..i + 4]);
        }
        assert_eq!(&output.pixels[..4], &image.pixels[..4]);
    }
}
#[test]
fn serial_drag_cancel_threshold_and_independent_rotated_handles_survive_session_and_history() {
    let mut e = Editor::default();
    for force in [false, true] {
        e.begin_serial(p(40., 40.), 10., "Arial", None).unwrap();
        e.extend_constrained(p(42., 40.), force).unwrap();
        assert_eq!(e.draft().unwrap().serial_arrow_tip.is_some(), force);
        assert!(e.cancel());
        assert_eq!(e.serial_count(), 0);
    }
    e.begin_serial(p(40., 40.), 10., "Arial", None).unwrap();
    e.extend(p(55., 40.)).unwrap();
    assert!(e.commit());
    assert_eq!(e.serial_count(), 1);
    assert_eq!(e.elements()[0].points.len(), 2);
    e.rotate(0, 90.).unwrap();
    let source = e.elements()[0].clone();
    e.move_serial_handle(0, 1, p(75., 48.)).unwrap();
    assert_eq!(e.elements()[0].points, source.points);
    let world = |s: &crate::editor::Stroke| {
        let [c, t] = s.serial_handles().unwrap();
        crate::rotation::rotate(t, c, s.rotation)
    };
    let tip = world(&e.elements()[0]);
    e.move_serial_handle(0, 0, p(51., 48.)).unwrap();
    assert!(
        (world(&e.elements()[0]).x - tip.x).abs() < 0.001
            && (world(&e.elements()[0]).y - tip.y).abs() < 0.001
    );
    let again = reopen(&e);
    assert_eq!(
        again.elements()[0].serial_arrow_tip,
        e.elements()[0].serial_arrow_tip
    );
    assert_eq!(again.elements()[0].text, e.elements()[0].text);
    assert!(e.undo() && e.undo());
    assert_eq!(e.elements()[0].serial_arrow_tip, source.serial_arrow_tip);
}
#[test]
fn edge_magnifier_clips_outside_document_instead_of_stretching_border_pixels() {
    let _apt = Apartment::new().unwrap();
    let image = image::Image {
        width: 40,
        height: 20,
        pixels: (0..40 * 20)
            .flat_map(|i| {
                if i % 40 < 4 {
                    [0u8, 0, 255, 255]
                } else {
                    [255, 255, 255, 255]
                }
            })
            .collect::<Vec<_>>()
            .into(),
    };
    let mut e = Editor::default();
    e.begin(Tool::Magnifier, p(10., 0.)).unwrap();
    e.extend(p(30., 20.)).unwrap();
    assert!(e.commit());
    e.set_lens_zoom(0, 200).unwrap();
    e.move_lens_handle(0, 0, p(0., 10.)).unwrap();
    let out = export::decode_png(
        export::style::Snapshot::default()
            .encoded_png(&image, &e, 1.)
            .unwrap()
            .to_vec(),
    );
    // Source [-5, 5] maps [-5, 0] to dest [10, 20] (unpainted white base) and
    // [0, 4] red pixels to dest [20, 28]; unclipped DrawBitmap would stretch red into [10, 20].
    assert_eq!(
        &out.pixels[(10 * 40 + 15) * 4..(10 * 40 + 15) * 4 + 4],
        &[255, 255, 255, 255]
    );
    assert_eq!(
        &out.pixels[(10 * 40 + 22) * 4..(10 * 40 + 22) * 4 + 4],
        &[0, 0, 255, 255]
    );
}
