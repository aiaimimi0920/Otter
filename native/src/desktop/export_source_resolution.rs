//! 工作流源分辨率 PNG：显式权威底图与逻辑坐标分离，不改变普通导出尺寸合同。
use super::*;
use crate::crop_geometry::{Rect, Size};
use crate::image_projection::{Projection, Sampling};
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "export_base_draw.rs"]
mod base_draw;
#[path = "export_beautify.rs"]
mod beautify;
#[path = "export_source_content.rs"]
mod content;
#[path = "export_projection_history_mapping.rs"]
mod history_mapping;
#[path = "export_projection_legacy_content.rs"]
mod legacy_content;
#[path = "export_source_png.rs"]
mod png;
#[path = "export_source_sample.rs"]
mod source_sample;
pub(super) use base_draw::draw;
#[path = "export_frame_border.rs"]
pub(in crate::desktop) mod frame_border;
#[path = "export_frame_clip.rs"]
pub(in crate::desktop) mod frame_clip;
#[path = "export_source_blur.rs"]
pub(in crate::desktop) mod source_blur;

pub(in crate::desktop) struct Frame {
    logical: Size,
    output: Option<(u32, u32)>,
    base: Option<D2D_RECT_F>,
    base_clip: Option<D2D_RECT_F>,
    sampling: Sampling,
    beautify: Option<beautify::Settings>,
    projection: bool,
}
impl From<Size> for Frame {
    fn from(logical: Size) -> Self {
        Self {
            logical,
            output: None,
            base: None,
            base_clip: None,
            sampling: Sampling::Full,
            beautify: None,
            projection: false,
        }
    }
}
impl Frame {
    /// 发送专用准入必须在合成前完成；复用同一布局计算最终尺寸，不从 PNG 头猜测。
    pub(in crate::desktop) fn for_projection(
        mut self,
        image: &Image,
        editor: &Editor,
    ) -> Result<(Self, (u32, u32))> {
        self.projection = true;
        let layout = self.layout(image, editor)?;
        let output = png::Output::new(image, &self, &layout)?;
        Ok((self, output.size))
    }
    fn layout(&self, image: &Image, editor: &Editor) -> Result<Layout> {
        // 完整逻辑框不是裁剪，否则不同宽高比会错误使用 max-density。
        let crop = editor.annotation_frame().filter(|frame| {
            editor.crop_view().is_some()
                || frame.x != 0.0
                || frame.y != 0.0
                || frame.width != self.logical.width
                || frame.height != self.logical.height
        });
        Layout::for_frame(image, self, crop)
    }
    /// 从实际编码布局导出editor→协作view的平移；轴密度由View按basis尺寸统一处理。
    pub(in crate::desktop) fn edit_layout(
        &self,
        image: &Image,
        editor: &Editor,
    ) -> Result<crate::projection_edit::source::Layout> {
        let layout = self.layout(image, editor)?;
        let padding = self.beautify.as_ref().map_or(0, |value| value.padding) as f64;
        let px = padding * layout.visible.width / layout.output.0 as f64;
        let py = padding * layout.visible.height / layout.output.1 as f64;
        crate::projection_edit::source::Layout::new(
            layout.visible.width + 2.0 * px,
            layout.visible.height + 2.0 * py,
            px - layout.visible.x,
            py - layout.visible.y,
        )
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid collaboration export layout"))
    }
    pub(in crate::desktop) fn with_beautify(mut self, edit: &serde_json::Value) -> Result<Self> {
        self.beautify = beautify::Settings::read(edit)?;
        Ok(self)
    }
    pub(in crate::desktop) fn restored(
        image: &Image,
        logical: Size,
        crop: Option<crate::session_crop::Restored>,
    ) -> Result<Self> {
        // 持久裁剪的源坐标不能用按当前窗口非等比归一化后的 Editor 坐标代替。
        if let Some(crop) = crop {
            crop.editor_crop()
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored crop"))?;
        }
        let output = crop.map(|crop| {
            let density = (image.width as f64 / crop.crop.source.width)
                .max(image.height as f64 / crop.crop.source.height);
            (
                (crop.crop.rect.width * density).round().max(1.0) as u32,
                (crop.crop.rect.height * density).round().max(1.0) as u32,
            )
        });
        if output.is_some_and(|size| checked_image_bytes(size.0, size.1).is_none()) {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid restored source output size",
            ));
        }
        Ok(Self {
            logical,
            output,
            // logical 已是当前非等比裁剪 viewport。Layout 在此框内 uniform contain，
            // 不能先 contain 到旧 sourceSize 再逐轴拉伸自然图片。
            base: None,
            base_clip: None,
            sampling: Sampling::Full,
            beautify: None,
            projection: false,
        })
    }
}
#[path = "export_source_erase.rs"]
pub(in crate::desktop) mod source_erase;

pub(super) struct Layout {
    pub output: (u32, u32),
    pub visible: Rect,
    pub base: D2D_RECT_F,
    pub base_clip: Option<D2D_RECT_F>,
    pub sampling: Sampling,
    pub interpolation: D2D1_BITMAP_INTERPOLATION_MODE,
    pub border: Option<frame_border::Border>,
    pub radius: f32,
}
impl Layout {
    fn for_frame(image: &Image, frame: &Frame, crop: Option<Rect>) -> Result<Self> {
        let mut layout = Self::new(image, frame.logical, crop)?;
        if let Some(output) = frame.output {
            if checked_image_bytes(output.0, output.1).is_none() {
                return Err(Error::new(E_INVALIDARG, "Invalid source output size"));
            }
            layout.output = output;
        }
        if let Some(base) = frame.base {
            layout.base = base;
        }
        layout.base_clip = frame.base_clip;
        layout.sampling = frame.sampling;
        Ok(layout)
    }
    fn new(image: &Image, logical: Size, crop: Option<Rect>) -> Result<Self> {
        if !logical.width.is_finite()
            || !logical.height.is_finite()
            || logical.width <= 0.0
            || logical.height <= 0.0
            || logical.width > 8192.0
            || logical.height > 8192.0
            || checked_image_bytes(image.width, image.height) != Some(image.pixels.len())
        {
            return Err(Error::new(E_INVALIDARG, "Invalid source-resolution frame"));
        }
        let visible = crop.unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: logical.width,
            height: logical.height,
        });
        if ![visible.x, visible.y, visible.width, visible.height]
            .iter()
            .all(|value| value.is_finite())
            || visible.width <= 0.0
            || visible.height <= 0.0
            || visible.x < 0.0
            || visible.y < 0.0
            || visible.x + visible.width > logical.width + 0.000001
            || visible.y + visible.height > logical.height + 0.000001
        {
            return Err(Error::new(E_INVALIDARG, "Invalid source-resolution crop"));
        }
        let density =
            (image.width as f64 / logical.width).max(image.height as f64 / logical.height);
        let output = if crop.is_some() {
            (
                (visible.width * density).round().max(1.0) as u32,
                (visible.height * density).round().max(1.0) as u32,
            )
        } else {
            (image.width, image.height)
        };
        if checked_image_bytes(output.0, output.1).is_none() {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid source-resolution output dimensions",
            ));
        }
        let scale = (logical.width / image.width as f64).min(logical.height / image.height as f64);
        let width = image.width as f64 * scale;
        let height = image.height as f64 * scale;
        let x = (logical.width - width) / 2.0;
        let y = (logical.height - height) / 2.0;
        Ok(Self {
            output,
            visible,
            border: None,
            radius: 0.0,
            base_clip: None,
            sampling: Sampling::Full,
            base: D2D_RECT_F {
                left: x as f32,
                top: y as f32,
                right: (x + width) as f32,
                bottom: (y + height) as f32,
            },
            interpolation: D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
        })
    }
}

/// 仅在后台使用。image 必须是当前权威底图，调用方须在捕获时校验 retained source
/// 身份；不能用旧源覆盖底图擦除。editor 提供逻辑矢量/独立标注，layers.base 不取代 image。
pub(in crate::desktop) fn png_source_resolution(
    image: &Image,
    editor: &Editor,
    frame: Frame,
    opacity: f32,
    border: Option<frame_border::Border>,
    radius: f32,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    let check = || {
        if cancel.load(Ordering::Acquire) {
            Err(Error::new(
                windows::Win32::Foundation::E_ABORT,
                "Source-resolution export cancelled",
            ))
        } else {
            Ok(())
        }
    };
    check()?;
    if !radius.is_finite() || !(0.0..=8192.0).contains(&radius) {
        return Err(Error::new(E_INVALIDARG, "Invalid corner radius"));
    }
    if editor.draft().is_some() {
        return Err(Error::new(
            E_INVALIDARG,
            "Source-resolution export requires committed editor",
        ));
    }
    let mut layout = frame.layout(image, editor)?;
    let output = png::Output::new(image, &frame, &layout)?;
    layout.border = border;
    layout.radius = radius;
    let _apartment = Apartment::new()?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let bitmap = compose_layout(
        image,
        editor,
        &wic,
        opacity,
        crate::pixels::reserve,
        Some(&layout),
    )?;
    check()?;
    if let Some(settings) = &frame.beautify {
        let wrapped = beautify::apply(&bitmap.bitmap, settings, cancel)?;
        drop(bitmap);
        return output.raw(&wic, &wrapped, cancel);
    }
    output.bitmap(&wic, bitmap, cancel)
}

#[cfg(test)]
#[path = "export_source_projection_tests.rs"]
mod projection_tests;
#[cfg(test)]
#[path = "export_source_resolution_tests.rs"]
mod tests;
