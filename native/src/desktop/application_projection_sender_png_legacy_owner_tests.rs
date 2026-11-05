//! 真恢复后的nested/flip及retained失效换基准，仍经菜单/原Job/隔离TLS。
use super::*;
use crate::crop_geometry::Rect;
use serde_json::json;

fn pixel(x: usize, y: usize) -> [u8; 4] {
    [(x * 24) as u8, (y * 24) as u8, 32, 255]
}
pub(super) unsafe fn install(app: &mut Application, case: Case) -> Image {
    let working = matches!(case, Case::WorkingLegacy | Case::WorkingLegacySaveReopen);
    let height = if working { 4 } else { 8 };
    let natural = Image {
        width: 8,
        height,
        pixels: (0..8 * height as usize)
            .flat_map(|i| pixel(i % 8, i / 8))
            .collect::<Vec<_>>()
            .into(),
    };
    let cancel = AtomicBool::new(false);
    let png = crate::desktop::export::png_raw(&natural, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let unit = app.documents[0].state.unit_id.clone();
    let original = serde_json::to_vec(&json!({"stickers":[{
        "id":unit.as_str(),"x":0,"y":0,"w":8,"h":8,"src":url.as_str(),
        "imageEditState":{"cropRect":{"x":2,"y":0,"w":2,"h":height},
        "flippedX":matches!(case, Case::NaturalLegacyFlipped)}
    }],"links":[]}))
    .unwrap();
    png_legacy::restore_canvas(app, original.clone());
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(original.into());
    settle(app);
    if matches!(case, Case::NaturalLegacyNested) {
        app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .crop_to(Rect {
                x: 2.0,
                y: 2.0,
                width: 4.0,
                height: 4.0,
            })
            .unwrap();
        return Image {
            width: 4,
            height: 4,
            pixels: (0..16)
                .flat_map(|i| pixel(2 + i % 4 / 2, 2 + i / 4))
                .collect::<Vec<_>>()
                .into(),
        };
    }
    if matches!(case, Case::NaturalLegacyFlipped) {
        return Image {
            width: 8,
            height: 8,
            pixels: (0..64)
                .flat_map(|i| pixel(3 - i % 8 / 4, i / 8))
                .collect::<Vec<_>>()
                .into(),
        };
    }
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        let before = canvas.editor.layers().unwrap().clone();
        assert_eq!(before.size, (8, 8));
        let mut pixels = before.base.to_vec();
        // 工作纹理已contain自然8×4于逻辑8×8：自然首行位于工作y=2，而不是y=0。
        for y in 2..6 {
            pixels[(y * 8 + 2) * 4..(y * 8 + 2) * 4 + 4].copy_from_slice(&[7, 11, 13, 255]);
        }
        let mut after = before.clone();
        after.base = pixels.into();
        let revision = canvas.editor.revision();
        canvas
            .editor
            .replace_layers(revision, &before, after)
            .unwrap();
        assert!(!canvas
            .minified_source
            .as_ref()
            .unwrap()
            .matches(&canvas.editor));
        assert_ne!(
            &canvas.image.as_ref().unwrap().pixels[(2 * 8 + 2) * 4..(2 * 8 + 2) * 4 + 4],
            &[7, 11, 13, 255]
        );
        assert_eq!(
            canvas
                .editor
                .raster_crop_view()
                .unwrap()
                .legacy_pixel_region(),
            Some(Rect {
                x: 2.0,
                y: 2.0,
                width: 2.0,
                height: 4.0,
            })
        );
    }
    if matches!(case, Case::WorkingLegacySaveReopen) {
        settle(app);
        let saved = app
            .workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_source()
            .unwrap();
        png_legacy::restore_canvas(app, saved.to_vec());
    }
    Image {
        width: 8,
        height: 8,
        pixels: (0..64)
            .flat_map(|i| {
                if i % 8 < 4 {
                    [7, 11, 13, 255]
                } else {
                    pixel(3, i / 8 / 2)
                }
            })
            .collect::<Vec<_>>()
            .into(),
    }
}

#[test]
fn native_sender_restored_legacy_nested_crop_samples_current_region_exactly_once() {
    tls::run_source(0, Case::NaturalLegacyNested);
}
#[test]
fn native_sender_restored_legacy_flipped_pixels_never_reflect_source_region_twice() {
    tls::run_source(0, Case::NaturalLegacyFlipped);
}
#[test]
fn native_sender_legacy_working_texture_uses_rebased_crop_not_old_natural_or_host_pixels() {
    tls::run_source(0, Case::WorkingLegacy);
}
#[test]
fn native_sender_legacy_edited_working_texture_save_reopen_preserves_rebased_pixels() {
    tls::run_source(0, Case::WorkingLegacySaveReopen);
}
