//! 新Input必须穿过既有菜单/授权/后台/TLS/签名链；期望像素不调用渲染器。
use super::*;
use crate::{
    crop_geometry::{Rect, Size},
    desktop::canvas::minified_source::Origin,
    editor::Editor,
    erase_layers::Layers,
};
use serde_json::json;

fn record(
    unit: &crate::workspace_id::UnitId,
    width: f64,
    height: f64,
    edit: serde_json::Value,
) -> crate::session_source::Source {
    serde_json::to_vec(
        &json!({"stickers":[{"id":unit.as_str(),"x":0,"y":0,"w":width,"h":height,
        "imageEditState":edit}],"links":[]}),
    )
    .unwrap()
    .into()
}
fn logical_editor(width: u32, height: u32) -> Editor {
    Editor::with_layers(Layers {
        size: (width, height),
        base: [0, 255, 0, 255].repeat((width * height) as usize).into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
}
pub(super) unsafe fn install(app: &mut Application, case: Case) -> Image {
    match case {
        Case::NaturalNonuniform => nonuniform(app),
        Case::NaturalSavedLayout => saved_layout(app),
        Case::CrossNaturalHistory => cross_history(app),
        _ => unreachable!(),
    }
}

unsafe fn nonuniform(app: &mut Application) -> Image {
    let unit = app.documents[0].state.unit_id.clone();
    let natural = image(16, 16, [20, 40, 60, 255]);
    let original = record(
        &unit,
        8.0,
        4.0,
        json!({"cropRect":{"x":2,"y":2,"w":4,"h":4},
        "sourceSize":{"w":8,"h":8}}),
    );
    let mut canvas = app.documents[0].state.canvas.borrow_mut();
    canvas.editor = logical_editor(16, 8)
        .with_restored_size(Size {
            width: 16.0,
            height: 8.0,
        })
        .unwrap();
    canvas
        .editor
        .crop_to(Rect {
            x: 4.0,
            y: 2.0,
            width: 8.0,
            height: 4.0,
        })
        .unwrap();
    canvas.minified_source = Some(
        Source::new(natural, &canvas.editor)
            .with_source_logical(Size {
                width: 8.0,
                height: 8.0,
            })
            .with_origin(Origin {
                record: original,
                unit,
            }),
    );
    image(8, 8, [20, 40, 60, 255])
}

unsafe fn saved_layout(app: &mut Application) -> Image {
    let unit = app.documents[0].state.unit_id.clone();
    let original = record(&unit, 16.0, 4.0, json!({"flippedX":true}));
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas.editor = logical_editor(16, 4);
        canvas.minified_source = Some(
            Source::new(image(8, 8, [20, 40, 60, 255]), &canvas.editor).with_origin(Origin {
                record: original.clone(),
                unit: unit.clone(),
            }),
        );
    }
    let canonical = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(canonical).unwrap();
    let target = value["stickers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["id"] == unit.as_str())
        .unwrap();
    target["imageEditState"] = json!({"flippedX":false});
    target["stickerEditPropagation"] = json!({"upstreamSourceFrame":{"w":8,"h":4},
        "upstreamContentFrame":{"x":0,"y":0,"w":4,"h":4}});
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(serde_json::to_vec(&value).unwrap().into());
    settle(app);
    let saved = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap();
    let saved_value: serde_json::Value = serde_json::from_slice(saved).unwrap();
    let target = saved_value["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == unit.as_str())
        .unwrap();
    assert_eq!(target["imageEditState"]["flippedX"], false);
    assert!(!saved.same_storage(&original));
    let canvas = app.documents[0].state.canvas.borrow();
    assert!(canvas
        .minified_source
        .as_ref()
        .unwrap()
        .origin(&canvas.editor)
        .unwrap()
        .record
        .same_storage(&original));
    let mut expected = image(8, 8, [0; 4]);
    let mut pixels = expected.pixels.to_vec();
    for y in 0..8 {
        for x in 4..6 {
            pixels[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4].copy_from_slice(&[20, 40, 60, 255]);
        }
    }
    expected.pixels = pixels.into();
    expected
}

unsafe fn cross_history(app: &mut Application) -> Image {
    app.documents[0]
        .state
        .canvas
        .borrow_mut()
        .replace_image(image(16, 4, [0, 255, 0, 255]))
        .unwrap();
    let index = app.documents.len();
    let preview = image(8, 4, [20, 40, 60, 255]);
    app.add_state(State::new(Some(preview.clone()), true).unwrap(), false)
        .unwrap();
    let unit = app.documents[index].state.unit_id.clone();
    let original = record(
        &unit,
        8.0,
        4.0,
        json!({"contentEraseStrokes":[
        {"width":2,"points":[{"x":2.25,"y":2.25}]}]}),
    );
    {
        let mut canvas = app.documents[index].state.canvas.borrow_mut();
        canvas.minified_source = Some(
            Source::new(image(16, 8, [20, 40, 60, 255]), &canvas.editor).with_origin(Origin {
                record: original,
                unit: unit.clone(),
            }),
        );
    }
    settle(app);
    let frame = crate::desktop::display_frame::Frame::capture(
        unit,
        preview,
        Some(&app.documents[index].state.canvas.borrow().editor),
    );
    *app.documents[0].state.resolved_display.borrow_mut() = Some(frame);
    let mut expected = vec![0; 16 * 8 * 4];
    for y in 0..8usize {
        for x in 4..12usize {
            let native_x = 2 * (x - 4) + 1; // 16自然像素contain到目标8输出像素；最近邻取像素中心。
            let mut removed = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let hx = (native_x as f64 + (sx as f64 + 0.5) / 4.0) / 2.0;
                    let hy = (y as f64 + (sy as f64 + 0.5) / 4.0) / 2.0;
                    if (hx - 2.25).powi(2) + (hy - 2.25).powi(2) <= 1.0 {
                        removed += 1;
                    }
                }
            }
            for (channel, base) in [20u32, 40, 60, 255].iter().enumerate() {
                expected[(y * 16 + x) * 4 + channel] = ((base * (16 - removed) + 8) / 16) as u8;
            }
        }
    }
    Image {
        width: 16,
        height: 8,
        pixels: expected.into(),
    }
}

#[test]
fn native_sender_png_input_keeps_nonuniform_restored_crop_at_natural_density() {
    tls::run_source(0, Case::NaturalNonuniform);
}
#[test]
fn native_sender_png_input_uses_frozen_natural_layout_after_canonical_save() {
    tls::run_source(0, Case::NaturalSavedLayout);
}
#[test]
fn native_sender_png_input_maps_real_cross_unit_source_history_to_target_geometry() {
    tls::run_source(0, Case::CrossNaturalHistory);
}
