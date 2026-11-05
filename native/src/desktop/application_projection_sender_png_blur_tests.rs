//! 活动blur必须经真实菜单/TLS带上当前传播布局；oracle仅用独立Gaussian和source-over。
use super::*;
use crate::{
    blur_image::BlurImage,
    stroke_style::{Color, StrokeStyle},
};

pub(super) unsafe fn install(app: &mut Application) -> Image {
    let unit = app.documents[0].state.unit_id.clone();
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .replace_image(image(16, 16, [0, 255, 0, 255]))
            .unwrap();
        canvas
            .editor
            .set_style(StrokeStyle::new(64.0, Color(0, 0, 0)).unwrap());
        canvas
            .editor
            .begin_blur(
                Point { x: 8.0, y: 8.0 },
                BlurImage::new(16, 16, vec![0; 16 * 16 * 4].into(), 1).unwrap(),
            )
            .unwrap();
        assert!(canvas.editor.commit());
        canvas.minified_source = Some(Source::new(image(16, 16, [255, 0, 0, 255]), &canvas.editor));
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
        .find(|entry| entry["id"] == unit.as_str())
        .unwrap();
    target["stickerEditPropagation"] = serde_json::json!({"upstreamSourceFrame":{"w":16,"h":16},
        "upstreamContentFrame":{"x":8,"y":0,"w":8,"h":16}});
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(serde_json::to_vec(&value).unwrap().into());
    settle(app);

    // 完整16×16源contain到传播8×16内容，底图占[8,16)×[4,12)。
    // 64px点遮罩覆盖全图，blur之上原绘制器再做8%白wash，最后source-over到底图。
    let weight = |offset: i32| {
        if offset.abs() > 3 {
            return 0.0;
        }
        let gaussian = |x: i32| (-(x as f64).powi(2) / 2.0).exp();
        gaussian(offset) / (-3..=3).map(gaussian).sum::<f64>()
    };
    let mut expected = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let horizontal = (8..16).map(|q| weight(x - q)).sum::<f64>();
            let vertical = (4..12).map(|q| weight(y - q)).sum::<f64>();
            let blurred = (255.0 * horizontal * vertical).round();
            let blue = if (8..16).contains(&x) && (4..12).contains(&y) {
                255
            } else {
                (blurred * 0.92 + 255.0 * 0.08).round() as u8
            };
            expected.extend_from_slice(&[blue, 20, 20, blue]);
        }
    }
    Image {
        width: 16,
        height: 16,
        pixels: expected.into(),
    }
}

#[test]
fn native_sender_menu_active_blur_samples_propagated_natural_pixels_before_frame_decoration() {
    tls::run_source(0, Case::NaturalPropagatedBlur);
}
