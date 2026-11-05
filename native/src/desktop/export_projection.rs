//! 单向投送的显式权威底图 PNG；来源身份由活动窗口快照校验，不在后台重新选图。
use super::*;
use std::sync::atomic::AtomicBool;
#[path = "export_projection_geometry.rs"]
mod geometry;
#[path = "export_projection_input.rs"]
mod input;
#[path = "export_projection_frame.rs"]
mod source_frame;
#[path = "export_projection_target.rs"]
mod target;
pub(in crate::desktop) use geometry::Geometry;
pub(in crate::desktop) use input::{Baseline, Input};
pub(in crate::desktop) struct Presentation {
    pub opacity: f32,
    pub kind: crate::projection_delivery::sender::Kind,
    pub geometry: Option<Geometry>,
}
/// canonical 原文只在既有发送 worker 中读取；目标外观与像素来源独立。
#[cfg(test)]
pub(in crate::desktop) fn target_frame(
    source: &crate::session_source::Source,
    unit: &str,
    image: &Image,
    editor: &Editor,
    presentation: Presentation,
    cancel: &AtomicBool,
) -> Result<crate::projection_delivery::sender::Frame> {
    target_frame_with_input(
        source,
        unit,
        image,
        editor,
        presentation,
        Input::default(),
        cancel,
    )
}
/// 无矢量basis与普通PNG共用全部来源/几何/erase/外观；不能用截图或复制renderer代替。
pub(in crate::desktop) fn basis_frame(
    source: &crate::session_source::Source,
    unit: &str,
    image: &Image,
    editor: &Editor,
    presentation: Presentation,
    input: Input,
    cancel: &AtomicBool,
) -> Result<(
    crate::projection_delivery::sender::Frame,
    crate::projection_edit::source::Layout,
)> {
    if editor.draft().is_some() || editor.elements().iter().any(|stroke| stroke.blur.is_some()) {
        return Err(Error::new(
            E_INVALIDARG,
            "Active blur or draft cannot join collaboration",
        ));
    }
    render(
        source,
        unit,
        image,
        &editor.export_without_annotations(),
        Presentation {
            opacity: 1.0,
            ..presentation
        },
        input,
        cancel,
    )
}
/// Input只携带宿主已验证的待应用delta；网络、签名及迟到结果仍由原发送owner负责。
pub(in crate::desktop) fn target_frame_with_input(
    source: &crate::session_source::Source,
    unit: &str,
    image: &Image,
    editor: &Editor,
    presentation: Presentation,
    input: Input,
    cancel: &AtomicBool,
) -> Result<crate::projection_delivery::sender::Frame> {
    render(source, unit, image, editor, presentation, input, cancel).map(|(frame, _)| frame)
}
fn render(
    source: &crate::session_source::Source,
    unit: &str,
    image: &Image,
    editor: &Editor,
    presentation: Presentation,
    input: Input,
    cancel: &AtomicBool,
) -> Result<(
    crate::projection_delivery::sender::Frame,
    crate::projection_edit::source::Layout,
)> {
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Projection export cancelled",
        ));
    }
    let target = target::read(source, unit)?;
    let baseline = input
        .baseline
        .as_ref()
        .map(|baseline| target::read_baseline(&baseline.record, baseline.unit.as_str()))
        .transpose()?;
    if input.legacy_crop && !input.content && input.legacy_pixels.is_none() {
        return Err(Error::new(
            E_INVALIDARG,
            "Legacy crop requires pending content layout",
        ));
    }
    let fallback = if let Some(logical) = input.logical {
        Some(logical)
    } else if editor.full_logical_frame().is_none() && editor.layers().is_none() {
        presentation
            .geometry
            .map(|geometry| geometry.logical(target.hint))
            .transpose()?
            .or(target.logical)
    } else {
        target.logical
    };
    let logical = logical_size(image, editor, fallback);
    let border = source_resolution::frame_border::Border::read(&target.edit)?;
    let radius = source_resolution::frame_clip::radius(&target.edit)?;
    // 在翻转、局部效果及历史擦除分配之前准入源/内框/外框，不为过大来源先做昂贵工作。
    let mut geometry = source_frame::prepare(image, editor, logical, input.source_logical)?;
    if input.content || input.legacy_pixels.is_some() {
        // 布局只交换小几何，先确定legacy输出尺寸再准入；无需先分配翻转像素。
        geometry = target.content(image, editor, geometry, logical, &input, baseline.as_ref())?;
    }
    let (mut geometry, _) = geometry
        .with_beautify(&target.edit)?
        .for_projection(image, editor)?;
    let flipped = crate::raster_flip::image_axes(
        &image.pixels,
        (image.width, image.height),
        input.flip.0,
        input.flip.1,
        || cancel.load(std::sync::atomic::Ordering::Acquire),
    )
    .map_err(|error| {
        Error::new(
            match error {
                crate::raster_flip::Error::Cancelled => windows::Win32::Foundation::E_ABORT,
                crate::raster_flip::Error::BudgetExceeded => {
                    windows::Win32::Foundation::E_OUTOFMEMORY
                }
                _ => E_INVALIDARG,
            },
            "Cannot prepare projection orientation",
        )
    })?;
    let selected = Image {
        width: image.width,
        height: image.height,
        pixels: flipped,
    };
    // 先确定当前传播/crop取样，再重建活动blur；仍先于历史erase与最终外观。
    let prepared =
        source_resolution::source_blur::prepare_active(&selected, editor, &geometry, cancel)?;
    let selected = if input.historical_erase {
        let mapping = baseline
            .as_ref()
            .map(|baseline| baseline.history_mapping(&selected))
            .transpose()?;
        baseline.as_ref().unwrap_or(&target).erase(
            &selected,
            &mut geometry,
            editor.crop_view().map(|view| view.source.rect),
            mapping,
            cancel,
        )?
    } else {
        selected
    };
    let (geometry, (width, height)) = geometry.for_projection(&selected, editor)?;
    let layout = geometry.edit_layout(&selected, editor)?;
    let png = source_resolution::png_source_resolution(
        &selected,
        &prepared,
        geometry,
        presentation.opacity,
        border,
        radius,
        cancel,
    )?;
    Ok((
        crate::projection_delivery::sender::Frame {
            png,
            width,
            height,
            kind: presentation.kind,
        },
        layout,
    ))
}
#[cfg(test)]
pub(in crate::desktop) fn frame(
    image: &Image,
    editor: &Editor,
    opacity: f32,
    kind: crate::projection_delivery::sender::Kind,
    cancel: &AtomicBool,
) -> Result<crate::projection_delivery::sender::Frame> {
    let source = br#"{"stickers":[{"id":"projection-test"}]}"#.to_vec().into();
    target_frame(
        &source,
        "projection-test",
        image,
        editor,
        Presentation {
            opacity,
            kind,
            geometry: None,
        },
        cancel,
    )
}
fn logical_size(
    image: &Image,
    editor: &Editor,
    fallback: Option<crate::crop_geometry::Size>,
) -> crate::crop_geometry::Size {
    // 普通 Editor 没有显式 logical_size；其标注/裁剪仍以编辑底图坐标为准，
    // 不能在选中更大的 retained 原图后把自然像素误当成逻辑源框。
    let raster_size = editor.layers().map_or_else(
        || {
            fallback.unwrap_or(crate::crop_geometry::Size {
                width: image.width as f64,
                height: image.height as f64,
            })
        },
        |layers| crate::crop_geometry::Size {
            width: layers.size.0 as f64,
            height: layers.size.1 as f64,
        },
    );
    editor
        .full_logical_frame()
        .map_or(raster_size, |frame| crate::crop_geometry::Size {
            width: frame.width,
            height: frame.height,
        })
}
#[cfg(test)]
#[path = "export_projection_tests.rs"]
mod tests;
