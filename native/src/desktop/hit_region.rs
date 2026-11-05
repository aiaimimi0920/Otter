//! 用窗口区域限制跨进程鼠标命中；不依赖 HTTRANSPARENT 的同线程转发语义。
use super::State;
use crate::{
    alpha_region::{self, AlphaRegion},
    editor::Tool,
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

/// 区域缓存依赖实际绘制坐标系，不能把窗口客户区尺寸当作 viewport 的替代。
#[derive(Clone, Copy, Debug, PartialEq)]
struct Key {
    source: usize,
    revision: u64,
    pan: bool,
    client: (u32, u32),
    dpi: u32,
    viewport: crate::viewport::Viewport,
    minified: Option<(crate::session_minified::Views, crate::crop_geometry::Size)>,
}

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

pub(super) unsafe fn update(hwnd: HWND, state: &State) -> Result<()> {
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
        };
        let mut cache = state.hit_region.borrow_mut();
        if cache.key == Some(key) {
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
) -> Result<OwnedRegion> {
    let region = source_region(&cache.source.as_ref().unwrap().1, width, height, scale)?;
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

unsafe fn transformed_region(
    mask: &AlphaRegion,
    width: u32,
    height: u32,
    transform: XFORM,
) -> Result<OwnedRegion> {
    if mask.rectangles.is_empty() {
        return OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0));
    }
    let header = std::mem::size_of::<RGNDATAHEADER>();
    let bytes = header + mask.rectangles.len() * std::mem::size_of::<RECT>();
    // u32 对齐覆盖 RGNDATAHEADER/RECT，所有长度都来自有界矩形列表。
    let mut storage = vec![0u32; bytes.div_ceil(4)];
    let data = storage.as_mut_ptr().cast::<RGNDATA>();
    (*data).rdh = RGNDATAHEADER {
        dwSize: header as u32,
        iType: RDH_RECTANGLES,
        nCount: mask.rectangles.len() as u32,
        nRgnSize: (bytes - header) as u32,
        rcBound: RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        },
    };
    let output = storage.as_mut_ptr().cast::<u8>().add(header).cast::<RECT>();
    for (index, rect) in mask.rectangles.iter().enumerate() {
        output.add(index).write(RECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        });
    }
    OwnedRegion::checked(ExtCreateRegion(Some(&transform), bytes as u32, data))
}

#[path = "hit_region_annotations.rs"]
mod annotations;
use annotations::add_annotations;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_mask_retains_independent_layer_identity_without_gdi_handles() {
        let image = super::super::image::Image {
            width: 4,
            height: 2,
            pixels: vec![255; 32].into(),
        };
        let annotations: crate::pixels::Pixels = vec![255; 16].into();
        let region =
            HitRegion::prepare_cancellable(&image, Some(&annotations), (2, 2), || false).unwrap();
        assert!(region
            .source
            .as_ref()
            .unwrap()
            .0
            .same_storage(&image.pixels));
        assert!(region
            .annotations
            .as_ref()
            .unwrap()
            .0
            .same_storage(&annotations));
        assert_eq!(region.annotation_size, (2, 2));
        assert!(region.key.is_none());
        assert!(
            HitRegion::prepare_cancellable(&image, Some(&annotations), (2, 2), || true).is_err()
        );
    }
    #[test]
    fn region_key_invalidates_for_view_changes_without_document_or_window_changes() {
        let original = Key {
            source: 1,
            revision: 2,
            pan: true,
            client: (640, 480),
            dpi: 144,
            viewport: crate::viewport::Viewport {
                origin: crate::viewport::Point { x: -30.0, y: 20.0 },
                scale: 1.5,
            },
            minified: None,
        };
        let mut moved = original;
        moved.viewport.origin.x += 1.0;
        assert_ne!(original, moved);
        let mut zoomed = original;
        zoomed.viewport.scale = 2.0;
        assert_ne!(original, zoomed);
        let mut vertical = original;
        vertical.viewport.origin.y -= 1.0;
        assert_ne!(original, vertical);
        assert_eq!(original, Key { ..original });
    }

    #[test]
    fn independent_annotation_dimensions_map_native_mask_and_reject_wrong_stride() {
        let image = super::super::image::Image {
            width: 32,
            height: 32,
            pixels: vec![0; 4096].into(),
        };
        let mut pixels = vec![0; 8 * 4 * 4];
        pixels[(8 + 1) * 4 + 3] = 255;
        let pixels: crate::pixels::Pixels = pixels.into();
        let horizontal = HitRegion::prepare_sized(&image, Some(&pixels), (8, 4)).unwrap();
        let vertical = HitRegion::prepare_sized(&image, Some(&pixels), (4, 8)).unwrap();
        let crop = Some(crate::crop_geometry::Rect {
            x: 16.0,
            y: 8.0,
            width: 16.0,
            height: 16.0,
        });
        unsafe {
            let h = raster_region(&horizontal, 32, 32, 1.0, crop).unwrap();
            assert!(PtInRegion(h.0, 19, 14).as_bool());
            assert!(!PtInRegion(h.0, 30, 14).as_bool());
            let v = raster_region(&vertical, 32, 32, 1.0, crop).unwrap();
            assert!(PtInRegion(v.0, 22, 13).as_bool());
            assert!(!PtInRegion(v.0, 22, 20).as_bool());
        }
        assert!(HitRegion::prepare_sized(&image, Some(&pixels), (8, 8)).is_err());
        assert!(HitRegion::prepare_sized(&image, Some(&pixels), (0, 4)).is_err());
    }

    #[test]
    fn raster_annotation_has_native_hit_region_when_base_is_erased() {
        let image = super::super::image::Image {
            width: 16,
            height: 16,
            pixels: vec![0; 16 * 16 * 4].into(),
        };
        let mut pixels = vec![0; 16 * 16 * 4];
        pixels[(8 * 16 + 8) * 4 + 3] = 1;
        let annotations: crate::pixels::Pixels = pixels.into();
        let cache = HitRegion::prepare_layers(&image, Some(&annotations)).unwrap();
        unsafe {
            let region = raster_region(&cache, 16, 16, 1.0, None).unwrap();
            assert!(PtInRegion(region.0, 8, 8).as_bool());
            assert!(!PtInRegion(region.0, 2, 2).as_bool());
        }
        assert!(HitRegion::prepare(&image)
            .unwrap()
            .source
            .unwrap()
            .1
            .rectangles
            .is_empty());
    }
    #[test]
    fn cropped_overlay_hit_region_moves_independently_from_base_at_multiple_scales() {
        let mut base = vec![0; 32 * 32 * 4];
        for y in 24..28 {
            for x in 24..28 {
                base[(y * 32 + x) * 4 + 3] = 255;
            }
        }
        let image = super::super::image::Image {
            width: 32,
            height: 32,
            pixels: base.into(),
        };
        let mut overlay = vec![0; 32 * 32 * 4];
        for y in 4..12 {
            for x in 4..12 {
                overlay[(y * 32 + x) * 4 + 3] = 255;
            }
        }
        let pixels: crate::pixels::Pixels = overlay.into();
        let cache = HitRegion::prepare_layers(&image, Some(&pixels)).unwrap();
        let frame = crate::crop_geometry::Rect {
            x: 16.0,
            y: 4.0,
            width: 16.0,
            height: 8.0,
        };
        for scale in [1.0, 1.5, 2.0] {
            unsafe {
                let region = raster_region(&cache, 32, 32, scale, Some(frame)).unwrap();
                assert!(
                    PtInRegion(region.0, (20.0 * scale) as i32, (6.0 * scale) as i32).as_bool()
                );
                assert!(
                    !PtInRegion(region.0, (8.0 * scale) as i32, (8.0 * scale) as i32).as_bool()
                );
                // 底图仍位于原始位置；最终可见裁剪由 update 的统一 clip 负责。
                assert!(
                    PtInRegion(region.0, (26.0 * scale) as i32, (26.0 * scale) as i32).as_bool()
                );
                let reset = raster_region(&cache, 32, 32, scale, None).unwrap();
                assert!(PtInRegion(reset.0, (8.0 * scale) as i32, (8.0 * scale) as i32).as_bool());
                assert!(
                    !PtInRegion(reset.0, (20.0 * scale) as i32, (6.0 * scale) as i32).as_bool()
                );
            }
        }
        assert!(cache.annotations.as_ref().unwrap().0.same_storage(&pixels));
    }

    #[test]
    fn native_region_preserves_holes_and_scaled_sampling_border() {
        let mut pixels = vec![0; 12 * 12 * 4];
        pixels[(6 * 12 + 6) * 4 + 3] = 255;
        let mask = alpha_region::build(12, 12, &pixels).unwrap();
        unsafe {
            let region = source_region(&mask, 12, 12, 2.0).unwrap();
            assert!(!PtInRegion(region.0, 1, 1).as_bool());
            assert!(PtInRegion(region.0, 12, 12).as_bool());
            assert!(!PtInRegion(region.0, 20, 20).as_bool());
        }
    }

    #[test]
    fn annotation_bounds_remain_interactive_on_transparent_source() {
        let image = super::super::image::Image {
            width: 32,
            height: 32,
            pixels: vec![0; 32 * 32 * 4].into(),
        };
        let mut canvas = super::super::canvas::Canvas::new(Some(image)).unwrap();
        canvas.viewport.origin = crate::viewport::Point::default();
        canvas
            .editor
            .begin(Tool::Brush, crate::viewport::Point { x: 16.0, y: 16.0 })
            .unwrap();
        canvas.editor.commit();
        unsafe {
            let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
            add_annotations(&region, &canvas.editor, canvas.viewport.scale, 96).unwrap();
            assert!(PtInRegion(region.0, 16, 16).as_bool());
            assert!(!PtInRegion(region.0, 2, 2).as_bool());
        }
    }
}
