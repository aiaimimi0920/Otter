//! 用窗口区域限制跨进程鼠标命中；不依赖 HTTRANSPARENT 的同线程转发语义。
use super::State;
use crate::{
    alpha_region::{self, AlphaRegion},
    editor::Tool,
    image_projection::{Projection, Sampling},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};
#[path = "hit_region_minified.rs"]
mod minified;

#[derive(Default)]
pub(super) struct HitRegion {
    source: Option<(crate::pixels::Pixels, AlphaRegion)>,
    annotations: Option<(crate::pixels::Pixels, AlphaRegion)>,
    annotation_size: (u32, u32),
    key: Option<Key>,
    display_identity: Option<std::sync::Arc<()>>,
}

#[path = "hit_region_extension.rs"]
mod extension;
#[path = "hit_region_frame.rs"]
mod frame;
use frame::Key;

impl HitRegion {
    /// 网络 worker 已准备有界 alpha；保留独立本地标注缓存，不在 UI 重扫新底图。
    pub(super) fn replace_source(&mut self, pixels: crate::pixels::Pixels, mask: AlphaRegion) {
        self.source = Some((pixels, mask));
        self.key = None;
        self.display_identity = None;
    }
    pub(super) fn prepare(image: &super::image::Image) -> Result<Self> {
        Self::prepare_sized(image, None, (image.width, image.height))
    }
    #[cfg(test)]
    fn prepare_layers(
        image: &super::image::Image,
        annotations: Option<&crate::pixels::Pixels>,
    ) -> Result<Self> {
        Self::prepare_sized(image, annotations, (image.width, image.height))
    }
    fn prepare_sized(
        image: &super::image::Image,
        annotations: Option<&crate::pixels::Pixels>,
        annotation_size: (u32, u32),
    ) -> Result<Self> {
        Self::prepare_cancellable(image, annotations, annotation_size, || false)
    }
    pub(super) fn prepare_cancellable(
        image: &super::image::Image,
        annotations: Option<&crate::pixels::Pixels>,
        annotation_size: (u32, u32),
        cancelled: impl Fn() -> bool,
    ) -> Result<Self> {
        let mask = alpha_region::build_layers_cancellable(
            image.width,
            image.height,
            &image.pixels,
            None,
            &cancelled,
        )
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid pin hit mask"))?;
        if mask.conservative {
            eprintln!("native-hit-region: fragmented alpha uses bounded row spans");
        }
        Ok(Self {
            source: Some((image.pixels.clone(), mask)),
            annotations: annotations
                .map(|pixels| {
                    alpha_region::build_layers_cancellable(
                        annotation_size.0,
                        annotation_size.1,
                        pixels,
                        None,
                        &cancelled,
                    )
                    .map(|mask| (pixels.clone(), mask))
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid annotation hit mask"))
                })
                .transpose()?,
            key: None,
            display_identity: None,
            annotation_size,
        })
    }
}

struct OwnedRegion(HRGN);
impl Drop for OwnedRegion {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

impl OwnedRegion {
    fn checked(region: HRGN) -> Result<Self> {
        if region.is_invalid() {
            Err(Error::from_win32())
        } else {
            Ok(Self(region))
        }
    }
}

pub(super) use super::pin_window::preview::hit_update as update;
pub(super) unsafe fn update_visible(hwnd: HWND, state: &State) -> Result<()> {
    if state.canvas.borrow().minified.is_some() {
        return minified::update(hwnd, state);
    }
    // 生成区域时只借用模型；SetWindowRgn 可能同步发送窗口消息，调用前必须释放所有 RefCell。
    let prepared = {
        let canvas = state.canvas.borrow();
        if !canvas.pinned {
            return Ok(());
        }
        let display = state.resolved_display.borrow();
        let empty = crate::editor::Editor::default();
        let editor = display.as_ref().map_or(&canvas.editor, |frame| {
            frame.editor.as_deref().unwrap_or(&empty)
        });
        let committed = editor.layers().map(|layers| super::image::Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels: layers.base.clone(),
        });
        let annotations = editor
            .layers()
            .and_then(|layers| layers.annotations.as_ref());
        let Some(image) = display
            .as_ref()
            .map(|frame| &frame.image)
            .or(committed.as_ref())
            .or(canvas.image.as_ref())
        else {
            return Ok(());
        };
        let id = image.pixels.as_ptr() as usize;
        let annotation_size = editor
            .layers()
            .map_or((image.width, image.height), |layers| {
                layers.annotation_dimensions()
            });
        let dpi = GetDpiForWindow(hwnd);
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client)?;
        let key = Key {
            source: id,
            revision: editor.revision(),
            pan: canvas.tool == Tool::Pan,
            client: (client.right as u32, client.bottom as u32),
            dpi,
            viewport: canvas.viewport,
            minified: None,
            base_projection: editor.raster_base_projection(),
            style: canvas.frame_style,
            overlay: extension::identity(state),
        };
        let mut cache = state.hit_region.borrow_mut();
        if cache.key == Some(key) && state.pin_resize.rect.get().is_none() {
            return Ok(());
        }
        let region = if canvas.tool != Tool::Pan {
            OwnedRegion::checked(CreateRectRgn(0, 0, client.right, client.bottom))?
        } else {
            if cache
                .source
                .as_ref()
                .map(|(known, _)| known.as_ptr() as usize)
                != Some(id)
                || cache.annotation_size != annotation_size
                || match (&cache.annotations, annotations) {
                    (None, None) => false,
                    (Some((a, _)), Some(b)) => !a.same_storage(b),
                    _ => true,
                }
            {
                *cache = HitRegion::prepare_sized(image, annotations, annotation_size)?;
            }
            let region = raster_region(
                &cache,
                image.width,
                image.height,
                canvas.viewport.scale * dpi as f32 / 96.0,
                editor.annotation_frame(),
                key.base_projection,
            )?;
            add_annotations(&region, editor, canvas.viewport.scale, dpi)?;
            // 掩码及矢量区域先在源坐标缩放，再统一平移并与可见裁剪框相交。
            let factor = dpi as f32 / 96.0;
            let dx = (canvas.viewport.origin.x * factor).round() as i32;
            let dy = (canvas.viewport.origin.y * factor).round() as i32;
            if OffsetRgn(region.0, dx, dy) == RGN_ERROR {
                return Err(Error::from_win32());
            }
            if let Some(r) = editor.annotation_frame() {
                let scale = canvas.viewport.scale * factor;
                let clip = OwnedRegion::checked(CreateRectRgn(
                    (r.x as f32 * scale + dx as f32).floor() as i32,
                    (r.y as f32 * scale + dy as f32).floor() as i32,
                    ((r.x + r.width) as f32 * scale + dx as f32).ceil() as i32,
                    ((r.y + r.height) as f32 * scale + dy as f32).ceil() as i32,
                ))?;
                if CombineRgn(Some(region.0), Some(region.0), Some(clip.0), RGN_AND) == RGN_ERROR {
                    return Err(Error::from_win32());
                }
            }
            region
        };
        extension::add(&region, state, dpi)?;
        frame::apply(&region, key)?;
        cache.key = Some(key);
        region
    };
    if SetWindowRgn(hwnd, Some(prepared.0), false) == 0 {
        state.hit_region.borrow_mut().key = None;
        return Err(Error::from_win32());
    }
    // Windows 接管成功应用的 HRGN；失败时 RAII 删除仍归本进程所有的区域。
    std::mem::forget(prepared);
    Ok(())
}

unsafe fn source_region(
    mask: &AlphaRegion,
    width: u32,
    height: u32,
    scale: f32,
) -> Result<OwnedRegion> {
    transformed_region(
        mask,
        width,
        height,
        XFORM {
            eM11: scale,
            eM22: scale,
            ..Default::default()
        },
    )
}

/// 两层先各自映射到文档源坐标；调用方再统一应用 viewport 平移和裁剪。
unsafe fn raster_region(
    cache: &HitRegion,
    width: u32,
    height: u32,
    scale: f32,
    crop: Option<crate::crop_geometry::Rect>,
    projection: Option<Projection>,
) -> Result<OwnedRegion> {
    let mask = &cache.source.as_ref().unwrap().1;
    let region = match projection {
        Some(mapping) => projected_region(mask, (width, height), scale, mapping)?,
        None => source_region(mask, width, height, scale)?,
    };
    if let Some((_, mask)) = &cache.annotations {
        let frame = crop.unwrap_or(crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: width as f64,
            height: height as f64,
        });
        let overlay = transformed_region(
            mask,
            cache.annotation_size.0,
            cache.annotation_size.1,
            XFORM {
                eM11: frame.width as f32 / cache.annotation_size.0 as f32 * scale,
                eM22: frame.height as f32 / cache.annotation_size.1 as f32 * scale,
                eDx: frame.x as f32 * scale,
                eDy: frame.y as f32 * scale,
                ..Default::default()
            },
        )?;
        if CombineRgn(Some(region.0), Some(region.0), Some(overlay.0), RGN_OR) == RGN_ERROR {
            return Err(Error::from_win32());
        }
    }
    Ok(region)
}

unsafe fn projected_region(
    mask: &AlphaRegion,
    size: (u32, u32),
    scale: f32,
    mapping: Projection,
) -> Result<OwnedRegion> {
    let Sampling::Region(Projection { source, target }) =
        Projection::clipped(mapping.source, mapping.target, size)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid projected hit region"))?
    else {
        return OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0));
    };
    let edge = |value: f64| {
        // GDI regions accept signed 27-bit coordinates.
        (value * f64::from(scale))
            .round()
            .clamp(-67_108_864.0, 67_108_863.0) as i32
    };
    let bounds = RECT {
        left: edge(target.x),
        top: edge(target.y),
        right: edge(target.x + target.width),
        bottom: edge(target.y + target.height),
    };
    let map_x = |x: f64| edge(target.x + (x - source.x) / source.width * target.width);
    let map_y = |y: f64| edge(target.y + (y - source.y) / source.height * target.height);
    // Clip each alpha span before scaling. A tiny source region must not transform
    // distant, unrelated source pixels through an enormous affine matrix.
    let rects = mask.rectangles.iter().filter_map(|r| {
        let left = f64::from(r.left).max(source.x);
        let top = f64::from(r.top).max(source.y);
        let right = f64::from(r.right).min(source.x + source.width);
        let bottom = f64::from(r.bottom).min(source.y + source.height);
        if right <= left || bottom <= top {
            return None;
        }
        let r = RECT {
            left: map_x(left),
            top: map_y(top),
            right: map_x(right),
            bottom: map_y(bottom),
        };
        (r.right > r.left && r.bottom > r.top).then_some(r)
    });
    region_from_rects(mask.rectangles.len(), bounds, None, rects)
}

unsafe fn transformed_region(
    mask: &AlphaRegion,
    width: u32,
    height: u32,
    transform: XFORM,
) -> Result<OwnedRegion> {
    region_from_rects(
        mask.rectangles.len(),
        RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        },
        Some(&transform),
        mask.rectangles.iter().map(|rect| RECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }),
    )
}

unsafe fn region_from_rects(
    capacity: usize,
    bounds: RECT,
    transform: Option<&XFORM>,
    rects: impl Iterator<Item = RECT>,
) -> Result<OwnedRegion> {
    if capacity == 0 {
        return OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0));
    }
    let header = std::mem::size_of::<RGNDATAHEADER>();
    let stride = std::mem::size_of::<RECT>();
    let bytes = header + capacity * stride;
    // u32 对齐覆盖 RGNDATAHEADER/RECT，所有长度都来自有界矩形列表。
    let mut storage = vec![0u32; bytes.div_ceil(4)];
    let data = storage.as_mut_ptr().cast::<RGNDATA>();
    let output = storage.as_mut_ptr().cast::<u8>().add(header).cast::<RECT>();
    let mut count = 0;
    for rect in rects {
        assert!(count < capacity);
        output.add(count).write(rect);
        count += 1;
    }
    if count == 0 {
        return OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0));
    }
    (*data).rdh = RGNDATAHEADER {
        dwSize: header as u32,
        iType: RDH_RECTANGLES,
        nCount: count as u32,
        nRgnSize: (count * stride) as u32,
        rcBound: bounds,
    };
    OwnedRegion::checked(ExtCreateRegion(
        transform.map(std::ptr::from_ref),
        (header + count * stride) as u32,
        data,
    ))
}

#[path = "hit_region_annotations.rs"]
mod annotations;
use annotations::add_annotations;

#[cfg(test)]
#[path = "hit_region_tests.rs"]
mod tests;
