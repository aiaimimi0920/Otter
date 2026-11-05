//! 读取主干逻辑源框裁剪，不把窗口尺寸或 cropRect 误当作自然图片像素。
use crate::{
    crop_geometry::{self, Crop, Rect, Size},
    session_window::Window,
};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Restored {
    pub frame: Rect,
    pub crop: Crop,
    pub flipped_x: bool,
    pub flipped_y: bool,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Shape,
    Geometry,
}
#[derive(Deserialize)]
struct SourceSize {
    w: f64,
    h: f64,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Fields {
    crop_rect: Option<crate::session_window::Rect>,
    source_size: Option<SourceSize>,
    #[serde(default)]
    flipped_x: bool,
    #[serde(default)]
    flipped_y: bool,
}
impl Restored {
    /// 将持久的源逻辑框归一化到当前显示比例。标注只需加 rect.x/y，不能再次缩放。
    /// 不取整：像素缓冲的 ceil 尺寸不能反过来改变裁剪和矢量坐标。
    pub fn editor_crop(&self) -> Result<Crop, Error> {
        let full = self.reset_frame()?;
        let rect = Rect {
            x: self.crop.rect.x * (self.frame.width / self.crop.rect.width),
            y: self.crop.rect.y * (self.frame.height / self.crop.rect.height),
            width: self.frame.width,
            height: self.frame.height,
        };
        let crop = Crop {
            rect,
            source: Size {
                width: full.width,
                height: full.height,
            },
        };
        // 独立校验局部坐标；屏幕位置相消不能掩盖无效的局部偏移。
        crop_geometry::reset(rect, Some(crop)).map_err(|_| Error::Geometry)?;
        Ok(crop)
    }
    /// 仍在屏幕逻辑框中计算；安装 HWND 前由平台层执行 DPI/坐标转换。
    pub fn reset_frame(&self) -> Result<Rect, Error> {
        crop_geometry::reset(self.frame, Some(self.crop))
            .map(|change| change.frame)
            .map_err(|_| Error::Geometry)
    }
    pub fn image_placement(&self, width: u32, height: u32) -> Result<Rect, Error> {
        crop_geometry::image_placement(
            self.frame,
            self.crop,
            Size {
                width: width as f64,
                height: height as f64,
            },
        )
        .map_err(|_| Error::Geometry)
    }
}
pub fn restore(window: &Window<'_>) -> Result<Option<Restored>, Error> {
    let fields: Fields = match window.image_edit_state {
        Some(raw) => serde_json::from_str(raw.get()).map_err(|_| Error::Shape)?,
        None => return Ok(None),
    };
    // 主干缺任一字段就不启用裁剪；原始字段仍留在完整会话，不做降级写回。
    let (Some(crop), Some(source)) = (fields.crop_rect, fields.source_size) else {
        return Ok(None);
    };
    let frame = window.expanded_rect().ok_or(Error::Geometry)?;
    let restored = Restored {
        frame: Rect {
            x: frame.x,
            y: frame.y,
            width: frame.w,
            height: frame.h,
        },
        crop: Crop {
            rect: Rect {
                x: crop.x,
                y: crop.y,
                width: crop.w,
                height: crop.h,
            },
            source: Size {
                width: source.w,
                height: source.h,
            },
        },
        flipped_x: fields.flipped_x,
        flipped_y: fields.flipped_y,
    };
    restored.reset_frame()?;
    Ok(Some(restored))
}

/// 旧cropRect使用自然像素；恢复图片已翻转，故在这里转换到已物化像素的坐标。
/// 不把旧区域当作逻辑sourceSize，也不夹紧越界区域（越界部分应保持透明）。
pub fn legacy_pixels(window: &Window<'_>, size: (u32, u32)) -> Result<Option<Rect>, Error> {
    let Some(raw) = window.image_edit_state else {
        return Ok(None);
    };
    let fields: Fields = serde_json::from_str(raw.get()).map_err(|_| Error::Shape)?;
    if fields.source_size.is_some() {
        return Ok(None);
    }
    let Some(crop) = fields.crop_rect else {
        return Ok(None);
    };
    if !crop.valid() || size.0 == 0 || size.1 == 0 {
        return Err(Error::Geometry);
    }
    let rect = Rect {
        x: if fields.flipped_x {
            size.0 as f64 - crop.x - crop.w
        } else {
            crop.x
        },
        y: if fields.flipped_y {
            size.1 as f64 - crop.y - crop.h
        } else {
            crop.y
        },
        width: crop.w,
        height: crop.h,
    };
    if ![
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.x + rect.width,
        rect.y + rect.height,
    ]
    .iter()
    .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
    {
        return Err(Error::Geometry);
    }
    Ok(Some(rect))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn json(state: &str) -> String {
        format!(r#"{{"id":"u","x":100,"y":200,"w":200,"h":100,"imageEditState":{state}}}"#)
    }
    #[test]
    fn crop_uses_logical_source_size_and_retains_full_image_flip() {
        let json = json(
            r#"{"cropRect":{"x":50,"y":20,"w":100,"h":50},"sourceSize":{"w":400,"h":200},"flippedX":true,"contentEraseStrokes":[]}"#,
        );
        let window: Window<'_> = serde_json::from_str(&json).unwrap();
        let restored = restore(&window).unwrap().unwrap();
        assert_eq!(
            restored.reset_frame().unwrap(),
            Rect {
                x: 0.0,
                y: 160.0,
                width: 800.0,
                height: 400.0
            }
        );
        // 正方形自然图 contain-fit 到 800x400 的完整逻辑源框，再按裁剪偏移显示。
        assert_eq!(
            restored.image_placement(1000, 1000).unwrap(),
            Rect {
                x: 200.0,
                y: 160.0,
                width: 400.0,
                height: 400.0
            }
        );
        assert!(restored.flipped_x && !restored.flipped_y);
    }
    #[test]
    fn incomplete_crop_is_not_invented_and_invalid_geometry_is_rejected() {
        for state in ["{}", r#"{"cropRect":{"x":0,"y":0,"w":10,"h":10}}"#] {
            let text = json(state);
            let window: Window<'_> = serde_json::from_str(&text).unwrap();
            assert_eq!(restore(&window), Ok(None));
        }
        let text =
            json(r#"{"cropRect":{"x":0,"y":0,"w":0,"h":10},"sourceSize":{"w":100,"h":100}}"#);
        let window: Window<'_> = serde_json::from_str(&text).unwrap();
        assert_eq!(restore(&window), Err(Error::Geometry));
    }
    #[test]
    fn editor_space_preserves_fractional_nonuniform_scale_and_reset() {
        let text = r#"{"id":"u","x":-100,"y":-200,"w":75.5,"h":30.25,"imageEditState":{"cropRect":{"x":10,"y":5,"w":50,"h":25},"sourceSize":{"w":200,"h":100}}}"#;
        let window: Window<'_> = serde_json::from_str(text).unwrap();
        let restored = restore(&window).unwrap().unwrap();
        let crop = restored.editor_crop().unwrap();
        assert_eq!(
            crop.source,
            Size {
                width: 302.0,
                height: 121.0
            }
        );
        assert!((crop.rect.x - 15.1).abs() < 1e-10);
        assert!((crop.rect.y - 6.05).abs() < 1e-10);
        assert_eq!(crop.rect.width, 75.5);
        assert_eq!(crop.rect.height, 30.25);
        let reset = crop_geometry::reset(crop.rect, Some(crop)).unwrap();
        assert_eq!(
            reset.frame,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 302.0,
                height: 121.0
            }
        );
        // 原图在完整源框中的 contain-fit，再减裁剪偏移，与主干可见框投影一致。
        let local = crop_geometry::image_placement(
            crop.rect,
            crop,
            Size {
                width: 20.0,
                height: 20.0,
            },
        )
        .unwrap();
        let screen = restored.image_placement(20, 20).unwrap();
        assert!((screen.x - restored.frame.x - (local.x - crop.rect.x)).abs() < 1e-10);
        assert!((screen.y - restored.frame.y - (local.y - crop.rect.y)).abs() < 1e-10);
        assert_eq!(screen.width, local.width);
        assert_eq!(screen.height, local.height);
    }
    #[test]
    fn editor_space_rejects_scaled_overflow() {
        let restored = Restored {
            frame: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            crop: Crop {
                rect: Rect {
                    x: 1.0,
                    y: 1.0,
                    width: 0.00001,
                    height: 1.0,
                },
                source: Size {
                    width: 100.0,
                    height: 100.0,
                },
            },
            flipped_x: false,
            flipped_y: false,
        };
        assert_eq!(restored.editor_crop(), Err(Error::Geometry));
    }
}
