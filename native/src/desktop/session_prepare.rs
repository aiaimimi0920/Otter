//! 会话图像的整批后台准备；重复引用共享像素，失败不返回部分窗口。
use super::image::Image;
use crate::{session_assets::References, session_document::Document, workspace_id::UnitId};
use std::{
    collections::HashMap,
    fs::File,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
};
#[path = "session_editor_layers.rs"]
mod editor_layers;
#[path = "session_effect_source.rs"]
mod effect_source;
#[path = "session_image_flip.rs"]
mod image_flip;
#[path = "session_raster_editor.rs"]
mod raster_editor;

pub struct Images {
    pub(in crate::desktop) sticker: super::sticker_input::Config,
    pub(in crate::desktop) accept_upstream: bool,
    pub capture_original: Option<std::sync::Arc<File>>,
    pub unit: UnitId,
    pub source: Option<Image>,
    pub preview: Option<Image>,
    pub rasterized_annotations: Option<Image>,
    pub restored_preview_locked: bool,
}
impl Images {
    pub fn display(&self) -> Option<&Image> {
        self.preview.as_ref().or(self.source.as_ref())
    }
}

/// 后台调用；open_admitted 接收不可信资源字符串，调用者须先完成路径授权才能返回 File。
/// 不请求 URL，不创建 HWND，不修改工作区。成功结果只包含拥有的像素/身份，不借用会话文本。
pub fn images(
    document: &Document<'_>,
    cancel: &AtomicBool,
    mut open_admitted: impl FnMut(&str) -> Result<File>,
) -> Result<Vec<Images>> {
    check(cancel)?;
    if document.windows.len() > crate::workspace::MAX_UNITS
        || document.windows.len() != document.workspace.units().len()
    {
        return Err(Error::new(E_INVALIDARG, "Session window topology mismatch"));
    }
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(document.windows.len())
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate restored image records"))?;
    let mut cache = HashMap::new();
    let mut flips = image_flip::Cache::default();
    cache
        .try_reserve(document.windows.len() * 3)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate session image cache"))?;
    // Topology::restore 在同一窗口顺序下建立唯一的工作区身份。
    for (window, unit) in document.windows.iter().zip(document.workspace.units()) {
        check(cancel)?;
        if unit.as_str() != window.id {
            return Err(Error::new(E_INVALIDARG, "Session window topology mismatch"));
        }
        let references = References::from_window(window);
        let flags = image_flip::Flags::read(window)?;
        let source = load(references.source, &mut cache, cancel, &mut open_admitted)?;
        let preview = load(references.preview, &mut cache, cancel, &mut open_admitted)?;
        let source = flips.apply(references.source, source, flags, cancel)?;
        let preview = flips.apply(references.preview, preview, flags, cancel)?;
        let rasterized_annotations = load(
            references.rasterized_annotations,
            &mut cache,
            cancel,
            &mut open_admitted,
        )?;
        prepared.push(Images {
            sticker: if window.is_art() {
                Default::default()
            } else {
                super::sticker_input::Config::restore(window.params, cancel)?
            },
            accept_upstream: if window.is_art() {
                true
            } else {
                let propagation: Option<serde_json::Value> = window
                    .sticker_edit_propagation
                    .map(|raw| serde_json::from_str(raw.get()))
                    .transpose()
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid sticker propagation"))?;
                propagation
                    .as_ref()
                    .and_then(|v| v.get("acceptUpstream"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
            },
            capture_original: window
                .otter_native_capture_original_src
                .as_deref()
                .map(|source| {
                    crate::session_capture_original::restore(source, cancel).map_err(|error| {
                        Error::new(
                            if cancel.load(Ordering::Acquire) {
                                E_ABORT
                            } else {
                                E_INVALIDARG
                            },
                            error.to_string(),
                        )
                    })
                })
                .transpose()?,
            unit: unit.clone(),
            source,
            preview,
            rasterized_annotations,
            restored_preview_locked: references.restored_preview_locked,
        });
    }
    check(cancel)?;
    Ok(prepared)
}
/// 工作流选择其他节点/手动参数的底图时，仍经显式 Root 读取并应用目标的翻转。
/// 单张加载后由调用者释放，不建立另一份全会话像素缓存。
pub(super) fn workflow_image(
    window: &crate::session_window::Window<'_>,
    reference: &str,
    root: Option<&super::session_asset_root::Root>,
    cancel: &AtomicBool,
) -> Result<Image> {
    let manual = if window.is_art() {
        None
    } else {
        use super::sticker_input::Asset;
        use serde_json::Value;
        let (_, manual) =
            crate::sticker_image_params::read_with_image(window.params, |values, image| {
                if values.get("image_path").and_then(Value::as_str) != Some(reference) {
                    return None;
                }
                let filename = values
                    .get("image_filename")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                Some(match image {
                    Some(image) => Asset::restore_checked(image, filename, cancel),
                    // An explicitly empty manual reference keeps its original rejection.
                    None => Asset::restore(reference, filename, cancel),
                })
            })
            .map_err(|e| Error::new(E_INVALIDARG, e))?;
        manual.transpose()?
    };
    let image = if let Some(asset) = manual {
        Some(asset.image.clone())
    } else {
        load(Some(reference), &mut HashMap::new(), cancel, &mut |path| {
            root.ok_or_else(|| {
                Error::new(
                    windows::Win32::Foundation::E_ACCESSDENIED,
                    "Workflow image requires admitted root",
                )
            })?
            .open_image(path)
        })?
    };
    image_flip::Cache::default()
        .apply(
            Some(reference),
            image,
            image_flip::Flags::read(window)?,
            cancel,
        )?
        .ok_or_else(|| Error::new(E_INVALIDARG, "Missing workflow image"))
}
fn load<'a>(
    reference: Option<&'a str>,
    cache: &mut HashMap<&'a str, Image>,
    cancel: &AtomicBool,
    open_admitted: &mut impl FnMut(&str) -> Result<File>,
) -> Result<Option<Image>> {
    check(cancel)?;
    let Some(reference) = reference.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Some(image) = cache.get(reference) {
        return Ok(Some(image.clone()));
    }
    let image = if reference.starts_with("data:") {
        super::session_image::inline(reference, cancel)?
    } else {
        // 打开策略留在调用者；原始 filePath 只作元数据，不可作为失败图片的隐式回退。
        let file = open_admitted(reference)?;
        check(cancel)?;
        super::session_image_file::load(file, cancel)?
    };
    check(cancel)?;
    cache.insert(reference, image.clone());
    Ok(Some(image))
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Session preparation cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_prepare_tests.rs"]
mod tests;
