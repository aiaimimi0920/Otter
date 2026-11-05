//! 已提交裁剪区域合成与 PNG 编码；复用标注渲染，不受宿主视口缩放和平移影响。
use super::{annotations, image::Image, Apartment};
#[cfg(test)]
#[path = "export_annotation_domain_tests.rs"]
mod annotation_domain_tests;
#[path = "export_drag.rs"]
mod drag;
#[path = "export_projection.rs"]
pub(super) mod projection;
#[path = "export_raw.rs"]
mod raw;
use crate::{
    editor::Editor,
    viewport::{checked_image_bytes, Point, Viewport},
};
pub(super) use drag::png_with_opacity;
#[path = "export_source_resolution.rs"]
pub(super) mod source_resolution;
pub(super) use raw::{png_checkpoint, png_raw, png_straight_bgra};
use std::{io::Write, path::Path};
#[cfg(test)]
pub(super) use tests::decode_png;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_FAIL, E_INVALIDARG, HGLOBAL},
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Imaging::*,
        },
        System::Com::{StructuredStorage::CreateStreamOnHGlobal, *},
    },
};

#[cfg(test)]
fn save_new(path: &Path, image: &Image, editor: &Editor) -> std::result::Result<(), String> {
    save_new_with_opacity(path, image, editor, 1.0)
}

pub(super) fn save_new_with_opacity(
    path: &Path,
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> std::result::Result<(), String> {
    save_allocated(image, editor, opacity, || {
        crate::file_naming::allocate::create_exact(path).map(|file| (file, path.to_path_buf()))
    })
    .map(|_| ())
}

pub(super) fn save_incremented(
    directory: &Path,
    stem: &str,
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> std::result::Result<std::path::PathBuf, String> {
    save_allocated(image, editor, opacity, || {
        crate::file_naming::allocate::create_unique_file(
            directory,
            stem,
            Some("png"),
            &std::sync::atomic::AtomicBool::new(false),
        )
    })
}

fn save_allocated(
    image: &Image,
    editor: &Editor,
    opacity: f32,
    allocate: impl FnOnce() -> std::io::Result<(std::fs::File, std::path::PathBuf)>,
) -> std::result::Result<std::path::PathBuf, String> {
    let _apartment = Apartment::new().map_err(|error| error.to_string())?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| error.to_string())?;
    let bitmap = compose_opacity(image, editor, &wic, opacity, crate::pixels::reserve)
        .map_err(|error| error.to_string())?;
    let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap).map_err(|error| error.to_string())?;
    drop(bitmap);
    // create_new 原子拒绝已有文件；不先截断目标，失败时也不删除任何既有用户内容。
    let (mut file, path) = allocate().map_err(|error| format!("Cannot create export: {error}"))?;
    copy_stream(&stream, size, &mut file).map_err(|error| {
        format!(
            "Export write incomplete; new output retained at {}: {error}",
            path.display()
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "Export write incomplete; new output retained at {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

pub(super) struct ComposedImage {
    // 字段顺序保证 WIC 缓冲先释放，预算后归还。
    pub(super) bitmap: IWICBitmap,
    _lease: crate::byte_budget::Lease,
}

/// 文件名和编码共用已提交裁剪尺寸；视口缩放、平移与窗口尺寸不参与。
pub(super) fn dimensions(image: &Image, editor: Option<&Editor>) -> Result<(u32, u32)> {
    let source = editor
        .and_then(Editor::layers)
        .map_or((image.width, image.height), |layers| layers.size);
    let size = editor
        .and_then(Editor::annotation_frame)
        .map_or(source, |rect| {
            (
                rect.width.round().max(1.0) as u32,
                rect.height.round().max(1.0) as u32,
            )
        });
    checked_image_bytes(size.0, size.1)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid cropped export dimensions"))?;
    Ok(size)
}

#[cfg(test)]
#[path = "export_budget_tests.rs"]
mod budget_tests;

pub(super) fn compose(
    image: &Image,
    editor: &Editor,
    wic: &IWICImagingFactory,
) -> Result<ComposedImage> {
    compose_with_budget(image, editor, wic, crate::pixels::reserve)
}

pub(super) fn compose_with_budget(
    image: &Image,
    editor: &Editor,
    wic: &IWICImagingFactory,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<ComposedImage> {
    compose_opacity(image, editor, wic, 1.0, reserve)
}

fn compose_opacity(
    image: &Image,
    editor: &Editor,
    wic: &IWICImagingFactory,
    opacity: f32,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<ComposedImage> {
    compose_layout(image, editor, wic, opacity, reserve, None)
}
fn compose_layout(
    image: &Image,
    editor: &Editor,
    wic: &IWICImagingFactory,
    opacity: f32,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
    layout: Option<&source_resolution::Layout>,
) -> Result<ComposedImage> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(Error::new(E_INVALIDARG, "Invalid base image opacity"));
    }
    // 编辑器已拥有栅格历史时，它的已提交快照才是导出权威；旧宿主 image 仅作兼容输入。
    let current = editor
        .layers()
        .filter(|_| layout.is_none())
        .map(|layers| Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels: layers.base.clone(),
        });
    let image = current.as_ref().unwrap_or(image);
    if checked_image_bytes(image.width, image.height) != Some(image.pixels.len()) {
        return Err(Error::new(E_INVALIDARG, "Invalid export pixel dimensions"));
    }
    let crop = layout
        .map(|layout| layout.visible)
        .or_else(|| editor.annotation_frame());
    let (width, height) = match layout {
        Some(layout) => layout.output,
        None => dimensions(image, Some(editor))?,
    };
    let output_bytes = checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid cropped export dimensions"))?;
    let annotation_bytes = editor.layers().map_or(0, |layers| {
        layers.annotations.as_ref().map_or(0, |pixels| pixels.len())
    });
    let upload_bytes = image
        .pixels
        .len()
        .checked_add(annotation_bytes)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Export upload size overflow"))?;
    // 原图共享预算之外，WIC 输出与 D2D 上传副本同时存在；均先准入再分配。
    let output_lease = reserve(output_bytes).ok_or_else(|| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Export bitmap budget exhausted",
        )
    })?;
    let _upload_lease = reserve(upload_bytes).ok_or_else(|| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Export upload budget exhausted",
        )
    })?;
    unsafe {
        let output = wic.CreateBitmap(
            width,
            height,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapCacheOnLoad,
        )?;
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            dpiX: 96.0,
            dpiY: 96.0,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            ..Default::default()
        };
        let target = factory.CreateWicBitmapRenderTarget(&output, &properties)?;
        let bitmap = target.CreateBitmap(
            D2D_SIZE_U {
                width: image.width,
                height: image.height,
            },
            Some(image.pixels.as_ptr().cast()),
            image.width * 4,
            &D2D1_BITMAP_PROPERTIES {
                pixelFormat: properties.pixelFormat,
                dpiX: 96.0,
                dpiY: 96.0,
            },
        )?;
        let annotation_bitmap = editor
            .layers()
            .and_then(|layers| layers.annotations.as_ref())
            .map(|pixels| {
                let (aw, ah) = editor.layers().unwrap().annotation_dimensions();
                if checked_image_bytes(aw, ah) != Some(pixels.len()) {
                    return Err(Error::new(
                        E_INVALIDARG,
                        "Invalid raster annotation dimensions",
                    ));
                }
                target.CreateBitmap(
                    D2D_SIZE_U {
                        width: aw,
                        height: ah,
                    },
                    Some(pixels.as_ptr().cast()),
                    aw * 4,
                    &D2D1_BITMAP_PROPERTIES {
                        pixelFormat: properties.pixelFormat,
                        dpiX: 96.0,
                        dpiY: 96.0,
                    },
                )
            })
            .transpose()?;
        let brush = target.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 0.96,
                g: 0.88,
                b: 0.24,
                a: 1.0,
            },
            None,
        )?;
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        if let Some(crop) = crop {
            // 底图和矢量保留源坐标；栅格标注另用当前可见框作为目标矩形。
            // 四舍五入后的输出尺寸按主干合同分别缩放两轴。
            let mut transform = Default::default();
            target.GetTransform(&mut transform);
            transform.M11 = width as f32 / crop.width as f32;
            transform.M22 = height as f32 / crop.height as f32;
            transform.M31 = -(crop.x as f32) * transform.M11;
            transform.M32 = -(crop.y as f32) * transform.M22;
            target.SetTransform(&transform);
        }
        let clip = source_resolution::frame_clip::push(&target, &factory, layout, &reserve)?;
        source_resolution::draw(&target, &bitmap, layout, opacity);
        if let Some(layout) = layout {
            if let Some(border) = &layout.border {
                border.draw(&target, layout.visible)?;
            }
        }
        if let Some(bitmap) = annotation_bitmap {
            // 主干完整绘制该层到可见框；不能复用底图的源框裁剪。
            let destination = crop
                .map(|r| D2D_RECT_F {
                    left: r.x as f32,
                    top: r.y as f32,
                    right: (r.x + r.width) as f32,
                    bottom: (r.y + r.height) as f32,
                })
                .unwrap_or(D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: image.width as f32,
                    bottom: image.height as f32,
                });
            target.DrawBitmap(
                &bitmap,
                Some(&destination),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
        let drawing = annotations::draw_committed(
            &target,
            &brush,
            editor,
            Viewport {
                origin: Point::default(),
                scale: 1.0,
            },
            &mut super::effect_cache::Cache::default(),
        );
        drop(clip);
        let finished = target.EndDraw(None, None);
        drawing?;
        finished?;
        Ok(ComposedImage {
            bitmap: output,
            _lease: output_lease,
        })
    }
}

#[cfg(test)]
fn encode_stream(image: &Image, editor: &Editor) -> Result<(IStream, u64)> {
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let bitmap = compose(image, editor, &wic)?;
        encode_bitmap(&wic, &bitmap.bitmap)
    }
}

fn encode_bitmap(wic: &IWICImagingFactory, bitmap: &IWICBitmap) -> Result<(IStream, u64)> {
    unsafe {
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        let encoder = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        encoder.CreateNewFrame(&mut frame, std::ptr::null_mut())?;
        let frame = frame.ok_or_else(|| Error::new(E_FAIL, "PNG frame missing"))?;
        frame.Initialize(None)?;
        let (mut width, mut height) = (0, 0);
        bitmap.GetSize(&mut width, &mut height)?;
        frame.SetSize(width, height)?;
        let mut format = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut format)?;
        frame.WriteSource(bitmap, std::ptr::null())?;
        frame.Commit()?;
        encoder.Commit()?;
        let mut stat = STATSTG::default();
        stream.Stat(&mut stat, STATFLAG_NONAME)?;
        if stat.cbSize == 0 || stat.cbSize > 257 * 1024 * 1024 {
            return Err(Error::new(E_FAIL, "PNG output exceeds export budget"));
        }
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        Ok((stream, stat.cbSize))
    }
}

/// 两种格式使用同一个合成快照，不重复绘制；不写文件或访问系统剪贴板。
#[cfg(test)]
pub(super) fn clipboard_payload(
    image: &Image,
    editor: &Editor,
) -> Result<super::clipboard_bitmap::Payload> {
    clipboard_payload_with_opacity(image, editor, 1.0)
}

pub(super) fn clipboard_payload_with_opacity(
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> Result<super::clipboard_bitmap::Payload> {
    let _apartment = Apartment::new()?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let bitmap = compose_opacity(image, editor, &wic, opacity, crate::pixels::reserve)?;
    let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap)?;
    let dibv5 = super::clipboard_bitmap::encode(&wic, &bitmap.bitmap)?;
    drop(bitmap);
    Ok(super::clipboard_bitmap::Payload {
        png: stream_pixels(stream, size)?,
        dibv5,
    })
}

fn stream_pixels(stream: IStream, size: u64) -> Result<crate::pixels::Pixels> {
    let size = usize::try_from(size).map_err(|_| Error::new(E_FAIL, "PNG size overflow"))?;
    let mut lease = crate::pixels::reserve(size).ok_or_else(|| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "PNG clipboard budget exhausted",
        )
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Cannot allocate PNG clipboard buffer",
        )
    })?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "PNG capacity exceeds budget",
        ));
    }
    copy_stream(&stream, size as u64, &mut bytes).map_err(|error| Error::new(E_FAIL, error))?;
    Ok(crate::pixels::Pixels::from_reserved(bytes, lease))
}

/// Loom 内联资源复用已提交快照合成，不额外生成剪贴板 DIB；返回实际裁剪输出尺寸。
pub(super) fn png_payload(
    image: &Image,
    editor: &Editor,
) -> Result<(crate::pixels::Pixels, u32, u32)> {
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let bitmap = compose(image, editor, &wic)?;
    let (mut width, mut height) = (0, 0);
    unsafe {
        bitmap.bitmap.GetSize(&mut width, &mut height)?;
    }
    let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap)?;
    if size > 64 * 1024 * 1024 {
        return Err(Error::new(E_FAIL, "Loom PNG exceeds input transfer budget"));
    }
    drop(bitmap);
    Ok((stream_pixels(stream, size)?, width, height))
}

fn copy_stream(
    stream: &IStream,
    mut remaining: u64,
    output: &mut impl Write,
) -> std::result::Result<(), String> {
    // 仅保留固定写入缓冲；不再复制整个 PNG，编码器/合成位图此时已经释放。
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        let requested = remaining.min(buffer.len() as u64) as u32;
        let mut read = 0;
        unsafe {
            stream
                .Read(buffer.as_mut_ptr().cast(), requested, Some(&mut read))
                .ok()
        }
        .map_err(|error| error.to_string())?;
        if read == 0 || read > requested {
            return Err("Incomplete PNG stream".into());
        }
        output
            .write_all(&buffer[..read as usize])
            .map_err(|error| error.to_string())?;
        remaining -= u64::from(read);
    }
    Ok(())
}

#[cfg(test)]
fn encode(image: &Image, editor: &Editor) -> Result<Vec<u8>> {
    let (stream, size) = encode_stream(image, editor)?;
    let mut bytes = Vec::new();
    copy_stream(&stream, size, &mut bytes).map_err(|error| Error::new(E_FAIL, error))?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "export_linear_tests.rs"]
mod linear_tests;
#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "crop_export_tests.rs"]
mod crop_tests;

#[cfg(test)]
#[path = "opacity_tests.rs"]
mod opacity_tests;

#[cfg(test)]
#[path = "shape_tests.rs"]
mod shape_tests;

#[cfg(test)]
#[path = "dash_tests.rs"]
mod dash_tests;

#[cfg(test)]
#[path = "mosaic_tests.rs"]
mod mosaic_tests;

#[cfg(test)]
#[path = "blur_tests.rs"]
mod blur_tests;

#[cfg(test)]
#[path = "effect_order_tests.rs"]
mod effect_order_tests;

#[cfg(test)]
#[path = "effect_preview_tests.rs"]
mod effect_preview_tests;

#[cfg(test)]
#[path = "raster_export_tests.rs"]
mod raster_export_tests;

#[cfg(test)]
#[path = "export_base_opacity_tests.rs"]
mod base_opacity_tests;
