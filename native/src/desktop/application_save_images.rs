//! 保存底图与独立栅格标注层；不合成矢量、不烘焙可见框裁剪或窗口透明度。
use crate::{editor::Editor, pixels::Pixels};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{Error, Result};
use windows::Win32::Foundation::{E_ABORT, E_INVALIDARG};

pub struct Png {
    pub bytes: Pixels,
    pub width: u32,
    pub height: u32,
}
pub struct Rasters {
    pub base: Png,
    pub annotations: Option<Png>,
}
pub struct InlineImage {
    pub url: crate::loom_base64::Encoded,
    pub width: u32,
    pub height: u32,
}
pub struct InlineRasters {
    pub base: InlineImage,
    pub annotations: Option<InlineImage>,
}
impl InlineRasters {
    /// 像素已物化翻转，安装新 PNG 时清除旧翻转标志；源框/裁剪/擦除仍须由调用方配套。
    pub fn install(
        self,
        mut fields: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<crate::session_write::WindowCandidate, crate::session_write::Error>
    {
        let mut state = match fields.remove("imageEditState") {
            None | Some(serde_json::Value::Null) => serde_json::Map::new(),
            Some(serde_json::Value::Object(state)) => state,
            Some(_) => return Err(crate::session_write::Error::Window),
        };
        state.insert("flippedX".into(), false.into());
        state.insert("flippedY".into(), false.into());
        fields.insert("imageEditState".into(), state.into());
        // 新资源替换旧像素；不能让旧预览或已清空的标注层在重开后复活。
        fields.remove("previewSrc");
        fields.remove("rasterizedAnnotationLayerSrc");
        let mut window = crate::session_write::WindowCandidate::new(fields);
        window.insert_encoded("src", self.base.url);
        if let Some(annotations) = self.annotations {
            window.insert_encoded("rasterizedAnnotationLayerSrc", annotations.url);
        }
        Ok(window)
    }
}
impl Png {
    pub fn into_data_url(self, cancel: &AtomicBool) -> Result<InlineImage> {
        let url =
            crate::loom_base64::encode_png_data_url(&self.bytes, cancel).map_err(|error| {
                let code = if error == crate::loom_base64::Error::Cancelled {
                    E_ABORT
                } else {
                    E_INVALIDARG
                };
                Error::new(code, "Cannot encode bounded session PNG Data URL")
            })?;
        Ok(InlineImage {
            url,
            width: self.width,
            height: self.height,
        })
    }
}
impl Rasters {
    /// 消费 PNG owner，成功后仅保留带预算的 Data URL；第二层失败时第一层也回收。
    pub fn into_data_urls(self, cancel: &AtomicBool) -> Result<InlineRasters> {
        let base = self.base.into_data_url(cancel)?;
        let annotations = self
            .annotations
            .map(|png| png.into_data_url(cancel))
            .transpose()?;
        Ok(InlineRasters { base, annotations })
    }
}

pub fn encode(editor: &Editor, cancel: &AtomicBool) -> Result<Option<Rasters>> {
    encode_source(editor, None, cancel)
}
impl super::Document {
    /// restored_source 已由 UI 捕获时的底图身份校验筛选；后台不从旧 src 猜测可复用性。
    pub fn encode_images(&self, cancel: &AtomicBool) -> Result<Option<Rasters>> {
        encode_source(&self.editor, self.restored_source.as_ref(), cancel)
    }
}
fn encode_source(
    editor: &Editor,
    source: Option<&crate::desktop::image::Image>,
    cancel: &AtomicBool,
) -> Result<Option<Rasters>> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Session image encoding cancelled"));
    }
    let Some(layers) = editor.layers() else {
        return source
            .map(|source| {
                Ok(Rasters {
                    base: png((source.width, source.height), &source.pixels, cancel)?,
                    annotations: None,
                })
            })
            .transpose();
    };
    if !layers.valid() {
        return Err(Error::new(E_INVALIDARG, "Invalid session image layers"));
    }
    let base = match source {
        Some(source) => png((source.width, source.height), &source.pixels, cancel)?,
        None => png(layers.size, &layers.base, cancel)?,
    };
    let annotations = layers
        .annotations
        .as_ref()
        .map(|pixels| png(layers.annotation_dimensions(), pixels, cancel))
        .transpose()?;
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Session image encoding cancelled"));
    }
    Ok(Some(Rasters { base, annotations }))
}
fn png((width, height): (u32, u32), pixels: &Pixels, cancel: &AtomicBool) -> Result<Png> {
    let image = crate::desktop::image::Image {
        width,
        height,
        pixels: pixels.clone(),
    };
    Ok(Png {
        bytes: crate::desktop::export::png_checkpoint(&image, cancel)?,
        width,
        height,
    })
}

#[cfg(test)]
#[path = "application_save_flip_tests.rs"]
mod flip_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_source_replaces_projected_base_but_not_current_overlay() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let source = crate::desktop::image::Image {
            width: 4,
            height: 2,
            pixels: [0, 255, 0, 255].repeat(8).into(),
        };
        let editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (2, 2),
            base: [0, 0, 255, 255].repeat(4).into(),
            annotation_size: Some((1, 1)),
            annotations: Some(vec![255, 0, 0, 255].into()),
        })
        .unwrap();
        let cancel = AtomicBool::new(false);
        let saved = encode_source(&editor, Some(&source), &cancel)
            .unwrap()
            .unwrap();
        let base = crate::desktop::export::decode_png(saved.base.bytes.to_vec());
        assert_eq!((base.width, base.height), (4, 2));
        assert_eq!(base.pixels, source.pixels);
        let overlay = crate::desktop::export::decode_png(saved.annotations.unwrap().bytes.to_vec());
        assert_eq!((overlay.width, overlay.height), (1, 1));
        assert_eq!(overlay.pixels.as_ref(), &[255, 0, 0, 255]);
        let fallback = encode_source(&editor, None, &cancel).unwrap().unwrap();
        let base = crate::desktop::export::decode_png(fallback.base.bytes.to_vec());
        assert_eq!(base.pixels, editor.layers().unwrap().base);
        let no_layers = encode_source(&Editor::default(), Some(&source), &cancel)
            .unwrap()
            .unwrap();
        assert_eq!((no_layers.base.width, no_layers.base.height), (4, 2));
        assert!(no_layers.annotations.is_none());
        assert!(encode_source(&editor, Some(&source), &AtomicBool::new(true)).is_err());
    }
    #[test]
    fn installed_resources_round_trip_through_complete_session_candidate() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let source = br#"{"stickers":[{"id":"one","x":0,"y":0,"w":2,"h":2,"previewSrc":"stale","rasterizedAnnotationLayerSrc":"stale","extension":123}],"links":[]}"#;
        let original = crate::session_document::decode(source).unwrap();
        let topology = crate::workspace_session::Topology::capture(&original.workspace);
        let cancel = AtomicBool::new(false);
        for overlay in [false, true] {
            let editor = Editor::with_layers(crate::erase_layers::Layers {
                size: (2, 2),
                base: [0, 0, 255, 255].repeat(4).into(),
                annotation_size: overlay.then_some((1, 1)),
                annotations: overlay.then(|| vec![255, 0, 0, 255].into()),
            })
            .unwrap();
            let mut images = Some(
                encode(&editor, &cancel)
                    .unwrap()
                    .unwrap()
                    .into_data_urls(&cancel)
                    .unwrap(),
            );
            let candidate =
                crate::session_write::assemble_owned(source, &topology, &cancel, |_, fields| {
                    images.take().unwrap().install(fields)
                })
                .unwrap();
            let restored = crate::session_document::decode(&candidate.bytes).unwrap();
            let window = &restored.windows[0];
            assert!(window.preview_src.is_none());
            let base =
                crate::desktop::session_image::inline(window.src.as_deref().unwrap(), &cancel)
                    .unwrap();
            assert_eq!((base.width, base.height), (2, 2));
            assert_eq!(base.pixels.as_ref(), editor.layers().unwrap().base.as_ref());
            if overlay {
                let image = crate::desktop::session_image::inline(
                    window.rasterized_annotation_layer_src.as_deref().unwrap(),
                    &cancel,
                )
                .unwrap();
                assert_eq!((image.width, image.height), (1, 1));
                assert_eq!(image.pixels.as_ref(), &[255, 0, 0, 255]);
            } else {
                assert!(window.rasterized_annotation_layer_src.is_none());
            }
            let value: serde_json::Value = serde_json::from_slice(&candidate.bytes).unwrap();
            assert_eq!(value["stickers"][0]["extension"], 123);
            assert_eq!(candidate.revision, 1);
        }
    }
    #[test]
    fn separate_layers_keep_distinct_sizes_and_do_not_include_vector_strokes() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (2, 2),
            base: [0, 0, 255, 255].repeat(4).into(),
            annotation_size: Some((1, 1)),
            annotations: Some(vec![255, 0, 0, 255].into()),
        })
        .unwrap();
        editor
            .begin(
                crate::editor::Tool::Brush,
                crate::viewport::Point::default(),
            )
            .unwrap();
        assert!(editor.commit());
        let stamp = editor.document_stamp();
        let output = encode(&editor, &AtomicBool::new(false)).unwrap().unwrap();
        assert_eq!((output.base.width, output.base.height), (2, 2));
        let overlay = output.annotations.as_ref().unwrap();
        assert_eq!((overlay.width, overlay.height), (1, 1));
        assert_eq!(&output.base.bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&overlay.bytes[..8], b"\x89PNG\r\n\x1a\n");
        let base = crate::desktop::export::decode_png(output.base.bytes.as_ref().to_vec());
        let annotations = crate::desktop::export::decode_png(overlay.bytes.as_ref().to_vec());
        assert_eq!(base.pixels.as_ref(), editor.layers().unwrap().base.as_ref());
        assert_eq!(
            annotations.pixels.as_ref(),
            editor
                .layers()
                .unwrap()
                .annotations
                .as_ref()
                .unwrap()
                .as_ref()
        );
        assert!(stamp.matches(&editor));
    }
}
