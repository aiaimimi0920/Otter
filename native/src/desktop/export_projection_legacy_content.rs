//! 已校验像素owner的legacy区域；复用传播框，仅替换自然像素到逻辑框的采样。
use super::*;
use crate::{editor::CropView, session_window::Window};
use serde_json::{json, Value};

impl Frame {
    pub(in crate::desktop) fn with_legacy_pixels(
        self,
        image: &Image,
        window: &Window<'_>,
        view: Option<CropView>,
        mut pixels: Rect,
        edit: &Value,
    ) -> Result<Self> {
        validate(pixels)?;
        // 整图作为临时legacy区域，取原with_content计算的传播框及目的框方向。
        // 真正源框已经位于post-flip像素，绝不能再按canonical反射一次。
        let modern = view.is_some_and(|view| view.legacy_pixel_region().is_none());
        let layout = json!({
            "cropRect": {"x":0,"y":0,"w":image.width,"h":image.height},
            "flippedX": !modern && edit["flippedX"].as_bool() == Some(true),
            "flippedY": !modern && edit["flippedY"].as_bool() == Some(true),
        });
        let mut frame = self.with_content(image, window, None, &layout)?;
        let Sampling::Region(projection) = frame.sampling else {
            return Err(Error::new(E_INVALIDARG, "Invalid legacy content frame"));
        };
        let mut destination = projection.target;
        if let Some(view) = view {
            let source = view.source;
            validate(source.rect)?;
            validate(Rect {
                x: 0.0,
                y: 0.0,
                width: source.source.width,
                height: source.source.height,
            })?;
            // 当前区域已经组合了自己的nested crop。反解完整区域，最终viewport裁一次；
            // 同时保留crop外邻域给活动blur。上游legacy不参与目标crop的这次反解。
            if !modern {
                // legacy完整view不是现代crop：保留原自然输出的逐轴密度，不能因
                // crop_view存在就用max-density把非等比逻辑框变成另一张更大的图。
                let output = (
                    (image.width as f64 * source.rect.width / source.source.width)
                        .round()
                        .max(1.0) as u32,
                    (image.height as f64 * source.rect.height / source.source.height)
                        .round()
                        .max(1.0) as u32,
                );
                checked_image_bytes(output.0, output.1)
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid legacy source output size"))?;
                frame.output = Some(output);
                let sx = pixels.width / source.rect.width;
                let sy = pixels.height / source.rect.height;
                pixels = Rect {
                    x: pixels.x - source.rect.x * sx,
                    y: pixels.y - source.rect.y * sy,
                    width: source.source.width * sx,
                    height: source.source.height * sy,
                };
                validate(pixels)?;
            }
            let sx = destination.width / source.rect.width;
            let sy = destination.height / source.rect.height;
            destination = Rect {
                x: source.rect.x + destination.x - source.rect.x * sx,
                y: source.rect.y + destination.y - source.rect.y * sy,
                width: source.source.width * sx,
                height: source.source.height * sy,
            };
            if let Some(clip) = frame.base_clip.as_mut() {
                clip.left += source.rect.x as f32;
                clip.right += source.rect.x as f32;
                clip.top += source.rect.y as f32;
                clip.bottom += source.rect.y as f32;
            }
        }
        validate(destination)?;
        frame.sampling = Projection::clipped(pixels, destination, (image.width, image.height))
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid explicit legacy projection"))?;
        Ok(frame)
    }
}

fn validate(rect: Rect) -> Result<()> {
    if rect.width <= 0.0
        || rect.height <= 0.0
        || [
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.x + rect.width,
            rect.y + rect.height,
        ]
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Invalid explicit legacy pixel region",
        ));
    }
    Ok(())
}
