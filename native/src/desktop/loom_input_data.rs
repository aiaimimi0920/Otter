//! 区分原生像素、保留 PNG 的正式图像及内联引用；解码/编码只在准备 worker 中执行。
use super::*;
pub(super) enum Data {
    Formal(crate::desktop::loom_image_value::ImageValue),
    Live(Arc<crate::live_frames::Frame<crate::live_texture::Texture>>),
    Raster {
        width: u32,
        height: u32,
        pixels: Pixels,
        editor: Box<Editor>,
    },
    Encoded {
        mime: String,
        base64: String,
    },
}

impl Data {
    pub(super) fn inline(uri: &str) -> Result<Self, Error> {
        if uri.len() > 1024 * 1024 {
            return Err(Error::Budget);
        }
        let (mime, encoded) = uri
            .strip_prefix("data:")
            .and_then(|value| value.split_once(";base64,"))
            .ok_or(Error::InvalidInput)?;
        if !matches!(
            mime,
            "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/bmp"
        ) || encoded.is_empty()
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self::Encoded {
            mime: mime.into(),
            base64: encoded.into(),
        })
    }
    pub(super) fn bytes(&self) -> Result<usize, Error> {
        match self {
            Self::Formal(image) => Ok(image.bytes()),
            Self::Live(frame) => {
                crate::viewport::checked_image_bytes(frame.resource.width, frame.resource.height)
                    .ok_or(Error::InvalidInput)
            }
            Self::Encoded { base64, .. } => Ok(base64.len()),
            Self::Raster {
                width,
                height,
                editor,
                ..
            } => {
                let (width, height) = editor
                    .annotation_frame()
                    .map(|frame| {
                        (
                            frame.width.round().max(1.0) as u32,
                            frame.height.round().max(1.0) as u32,
                        )
                    })
                    .unwrap_or_else(|| {
                        editor
                            .layers()
                            .map_or((*width, *height), |layers| layers.size)
                    });
                crate::viewport::checked_image_bytes(width, height).ok_or(Error::InvalidInput)
            }
        }
    }
    pub(super) fn prepare(self, cancel: &AtomicBool, actual: &mut usize) -> Result<Inline, Error> {
        let raster = match self {
            Self::Formal(image) => {
                *actual = actual
                    .checked_add(image.bytes())
                    .filter(|total| *total <= 256 * 1024 * 1024)
                    .ok_or(Error::Budget)?;
                return loom_inline::prepare_value(image, cancel).map_err(|error| {
                    if cancel.load(Ordering::Acquire) {
                        Error::Cancelled
                    } else {
                        Error::Prepare(error.to_string())
                    }
                });
            }
            Self::Live(frame) => {
                // 与编码共用有界输入 worker；UI 不执行 staging/Map 或像素复制。
                let image = crate::live_texture::readback::read(frame, cancel).map_err(|_| {
                    if cancel.load(Ordering::Acquire) {
                        Error::Cancelled
                    } else {
                        Error::InvalidInput
                    }
                })?;
                Self::Raster {
                    width: image.width,
                    height: image.height,
                    pixels: image.pixels,
                    editor: Box::default(),
                }
            }
            Self::Encoded { mime, base64 } => {
                if mime == "image/png" {
                    let image =
                        crate::desktop::loom_image_value::ImageValue::png_base64(&base64, cancel)
                            .map_err(|_| {
                            if cancel.load(Ordering::Acquire) {
                                Error::Cancelled
                            } else {
                                Error::InvalidInput
                            }
                        })?;
                    // 同一 Formal owner 计入像素+原 PNG 预算，再复用唯一的内联交付链。
                    return Self::Formal(image).prepare(cancel, actual);
                }
                let image =
                    super::super::super::loom_output_inline::decode_base64(&mime, &base64, cancel)
                        .map_err(|_| {
                            if cancel.load(Ordering::Acquire) {
                                Error::Cancelled
                            } else {
                                Error::InvalidInput
                            }
                        })?;
                Self::Raster {
                    width: image.width,
                    height: image.height,
                    pixels: image.pixels,
                    editor: Box::default(),
                }
            }
            raster => raster,
        };
        *actual = actual
            .checked_add(raster.bytes()?)
            .filter(|total| *total <= 256 * 1024 * 1024)
            .ok_or(Error::Budget)?;
        let Self::Raster {
            width,
            height,
            pixels,
            editor,
        } = raster
        else {
            unreachable!()
        };
        loom_inline::prepare(width, height, pixels, &editor, cancel).map_err(|error| {
            if cancel.load(Ordering::Acquire) {
                Error::Cancelled
            } else {
                Error::Prepare(error.to_string())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raster_budget_uses_logical_output_after_reset_and_undo() {
        let pixels: Pixels = vec![255; 128].into();
        let editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (8, 4),
            base: pixels.clone(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap()
        .with_restored_crop(crate::crop_geometry::Crop {
            rect: crate::crop_geometry::Rect {
                x: 1.0,
                y: 0.5,
                width: 2.25,
                height: 1.25,
            },
            source: crate::crop_geometry::Size {
                width: 7.25,
                height: 3.25,
            },
        })
        .unwrap();
        let mut data = Data::Raster {
            width: 8,
            height: 4,
            pixels,
            editor: Box::new(editor),
        };
        assert_eq!(data.bytes().unwrap(), 2 * 4);
        if let Data::Raster { editor, .. } = &mut data {
            assert!(editor.reset_crop());
        }
        assert_eq!(data.bytes().unwrap(), 7 * 3 * 4);
        if let Data::Raster { editor, .. } = &mut data {
            assert!(editor.undo());
        }
        assert_eq!(data.bytes().unwrap(), 2 * 4);
    }
}
