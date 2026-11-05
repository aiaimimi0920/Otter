//! 不同完整框的真实上游自然源：目标crop/reset/undo不得丢失原sourceSize密度。
use super::*;
use crate::{
    crop_geometry::{Rect, Size},
    desktop::canvas::minified_source::Origin,
};

fn pixel(x: usize, y: usize) -> [u8; 4] {
    [(x * 12) as u8, (y * 12) as u8, 32, 255]
}

pub(super) unsafe fn install(app: &mut Application, case: Case) -> Image {
    {
        let mut target = app.documents[0].state.canvas.borrow_mut();
        target
            .replace_image(image(16, 8, [0, 255, 0, 255]))
            .unwrap();
        target.editor = std::mem::take(&mut target.editor)
            .with_restored_size(Size {
                width: 16.0,
                height: 8.0,
            })
            .unwrap();
        target
            .editor
            .crop_to(Rect {
                x: 4.0,
                y: 2.0,
                width: 8.0,
                height: 4.0,
            })
            .unwrap();
        if matches!(case, Case::CrossNaturalReset | Case::CrossNaturalCropUndo) {
            assert!(target.editor.reset_crop());
            if matches!(case, Case::CrossNaturalCropUndo) {
                assert!(target.editor.undo());
                assert!(target.editor.crop_view().is_some());
            }
        }
    }
    let index = app.documents.len();
    let preview = image(8, 8, [255, 0, 0, 255]);
    app.add_state(State::new(Some(preview.clone()), true).unwrap(), false)
        .unwrap();
    let unit = app.documents[index].state.unit_id.clone();
    let origin = Origin {
        record: serde_json::to_vec(&serde_json::json!({
            "stickers":[{"id":unit.as_str(),"x":0,"y":0,"w":8,"h":8,
                "imageEditState":{"sourceSize":{"w":8,"h":8}}}],"links":[]
        }))
        .unwrap()
        .into(),
        unit: unit.clone(),
    };
    let natural = Image {
        width: 16,
        height: 16,
        pixels: (0..256)
            .flat_map(|i| pixel(i % 16, i / 16))
            .collect::<Vec<_>>()
            .into(),
    };
    {
        let mut upstream = app.documents[index].state.canvas.borrow_mut();
        upstream.editor = std::mem::take(&mut upstream.editor)
            .with_restored_size(Size {
                width: 8.0,
                height: 8.0,
            })
            .unwrap();
        upstream.minified_source = Some(
            Source::new(natural, &upstream.editor)
                .with_source_logical(Size {
                    width: 8.0,
                    height: 8.0,
                })
                .with_origin(origin),
        );
        assert_ne!(
            upstream.editor.full_logical_frame(),
            app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .full_logical_frame()
        );
    }
    settle(app);
    let frame = crate::desktop::display_frame::Frame::capture(
        unit,
        preview,
        Some(&app.documents[index].state.canvas.borrow().editor),
    );
    *app.documents[0].state.resolved_display.borrow_mut() = Some(frame);

    // 独立oracle：16×16自然图contain于目标16×8，底图占x=4..12。
    // crop的原sourceSize归一化为4×4，密度2，故输出8×8而非16×8。
    if !matches!(case, Case::CrossNaturalReset) {
        return Image {
            width: 8,
            height: 8,
            pixels: (0..64)
                .flat_map(|i| pixel(2 * (i % 8) + 1, i / 8 + 4))
                .collect::<Vec<_>>()
                .into(),
        };
    }
    let mut expected = vec![0; 16 * 16 * 4];
    for y in 0..16 {
        for x in 4..12 {
            expected[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4]
                .copy_from_slice(&pixel(2 * (x - 4) + 1, y));
        }
    }
    Image {
        width: 16,
        height: 16,
        pixels: expected.into(),
    }
}

#[test]
fn native_sender_cross_unit_different_full_frames_keep_target_crop_and_source_density() {
    tls::run_source(0, Case::CrossNaturalCrop);
}

#[test]
fn native_sender_cross_unit_target_reset_keeps_full_natural_output_and_letterbox() {
    tls::run_source(0, Case::CrossNaturalReset);
}

#[test]
fn native_sender_cross_unit_target_undo_recovers_crop_without_borrowing_upstream_geometry() {
    tls::run_source(0, Case::CrossNaturalCropUndo);
}
