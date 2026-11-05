//! 传播内容框只改变底图布局；标注、边框、输出分辨率仍属于完整可见框。
use super::*;
use crate::session_window::{Rect as SavedRect, Window};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Propagation {
    upstream_source_frame: Option<SourceSize>,
    upstream_content_frame: Option<SavedRect>,
}
#[derive(Deserialize)]
struct SourceSize {
    w: f64,
    h: f64,
}

impl Frame {
    pub(in crate::desktop) fn with_content(
        mut self,
        image: &Image,
        window: &Window<'_>,
        crop: Option<crate::session_crop::Restored>,
        edit: &serde_json::Value,
    ) -> Result<Self> {
        let target = window
            .expanded_rect()
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid propagated target frame"))?;
        let content = content_frame(window, target)?;
        let (x, y, width, height) = (content.x, content.y, content.w, content.h);
        let (mut left, mut top, w, h) = if let Some(crop) = crop {
            let visible = crop
                .editor_crop()
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid propagated crop"))?
                .rect;
            let sx = width / crop.crop.rect.width;
            let sy = height / crop.crop.rect.height;
            let viewport_w = crop.crop.source.width * sx;
            let viewport_h = crop.crop.source.height * sy;
            // 非等比裁剪先构造 viewport 再 contain；不能 contain 后拉伸。
            let fit = (viewport_w / image.width as f64).min(viewport_h / image.height as f64);
            let w = image.width as f64 * fit;
            let h = image.height as f64 * fit;
            self.base_clip = Some(rect(visible.x + x, visible.y + y, width, height)?);
            (
                visible.x + x + (viewport_w - w) / 2.0 - crop.crop.rect.x * sx,
                visible.y + y + (viewport_h - h) / 2.0 - crop.crop.rect.y * sy,
                w,
                h,
            )
        } else if let Some(legacy) = edit.get("cropRect").filter(|value| !value.is_null()) {
            // 无 sourceSize 的旧合同以自然像素取样，并拉伸到内容框。
            let legacy = SavedRect::deserialize(legacy)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid legacy pixel crop"))?;
            if !legacy.valid() {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Invalid legacy pixel crop geometry",
                ));
            }
            let mut source = Rect {
                x: legacy.x,
                y: legacy.y,
                width: legacy.w,
                height: legacy.h,
            };
            let mut destination = Rect {
                x,
                y,
                width,
                height,
            };
            if edit["flippedX"].as_bool() == Some(true) {
                source.x = image.width as f64 - source.x - source.width;
                destination.x = target.w - destination.x - destination.width;
            }
            if edit["flippedY"].as_bool() == Some(true) {
                source.y = image.height as f64 - source.y - source.height;
                destination.y = target.h - destination.y - destination.height;
            }
            self.base_clip = Some(rect(destination.x, destination.y, width, height)?);
            self.sampling =
                Projection::clipped(source, destination, (image.width, image.height))
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid legacy source projection"))?;
            return Ok(self);
        } else {
            let fit = (width / image.width as f64).min(height / image.height as f64);
            let w = image.width as f64 * fit;
            let h = image.height as f64 * fit;
            (x + (width - w) / 2.0, y + (height - h) / 2.0, w, h)
        };
        // 图片像素已经由恢复路径翻转。非裁剪还需围绕完整框镜像位置；
        // 裁剪则在图片自身中心翻转，内容框及裁剪偏移不镜像。
        if crop.is_none() {
            if edit["flippedX"].as_bool() == Some(true) {
                left = target.w - left - w;
                if let Some(clip) = self.base_clip.as_mut() {
                    (clip.left, clip.right) =
                        (target.w as f32 - clip.right, target.w as f32 - clip.left);
                }
            }
            if edit["flippedY"].as_bool() == Some(true) {
                top = target.h - top - h;
                if let Some(clip) = self.base_clip.as_mut() {
                    (clip.top, clip.bottom) =
                        (target.h as f32 - clip.bottom, target.h as f32 - clip.top);
                }
            }
        }
        self.base = Some(rect(left, top, w, h)?);
        Ok(self)
    }
}

fn content_frame(window: &Window<'_>, target: SavedRect) -> Result<SavedRect> {
    let fallback = SavedRect {
        x: 0.0,
        y: 0.0,
        w: target.w,
        h: target.h,
    };
    let Some(raw) = window.sticker_edit_propagation else {
        return Ok(fallback);
    };
    // 只解析固定大小几何，不复制传播历史或图片字符串。
    let value: Propagation = serde_json::from_str(raw.get())
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid propagated content frame"))?;
    let (Some(source), Some(content)) = (value.upstream_source_frame, value.upstream_content_frame)
    else {
        return Ok(fallback);
    };
    if source.w <= 0.0 || source.h <= 0.0 {
        return Ok(fallback);
    }
    if !source.w.is_finite() || !source.h.is_finite() || !content.valid() {
        return Err(Error::new(
            E_INVALIDARG,
            "Invalid propagated content geometry",
        ));
    }
    let scale = (target.w / source.w).min(target.h / source.h);
    Ok(SavedRect {
        x: content.x * scale + (target.w - source.w * scale) / 2.0,
        y: content.y * scale + (target.h - source.h * scale) / 2.0,
        w: content.w * scale,
        h: content.h * scale,
    })
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Result<D2D_RECT_F> {
    if ![x, y, w, h, x + w, y + h]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
        || w <= 0.0
        || h <= 0.0
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Propagated content exceeds geometry limits",
        ));
    }
    Ok(D2D_RECT_F {
        left: x as f32,
        top: y as f32,
        right: (x + w) as f32,
        bottom: (y + h) as f32,
    })
}
