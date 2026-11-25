//! Compose committed crop regions and encode PNG; reuse annotation rendering independently of host viewport zoom/pan.
use super::{annotations, image::Image, Apartment};
#[cfg(test)]
#[path = "export_annotation_domain_tests.rs"]
mod annotation_domain_tests;
#[cfg(test)]
#[path = "export_drag.rs"]
mod drag;
#[path = "export_paint.rs"]
mod paint;
#[path = "export_projection.rs"]
pub(super) mod projection;
#[path = "export_raw.rs"]
mod raw;
#[path = "export_style.rs"]
pub(super) mod style;
use crate::{
    editor::Editor,
    viewport::{checked_image_bytes, Point, Viewport},
};
#[cfg(test)]
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

#[cfg(test)]
pub(super) fn save_new_with_opacity(
    path: &Path,
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> std::result::Result<(), String> {
    save_allocated(image, editor, opacity, None, || {
        crate::file_naming::allocate::create_exact(path).map(|file| (file, path.to_path_buf()))
    })
    .map(|_| ())
}

fn save_allocated(
    image: &Image,
    editor: &Editor,
    opacity: f32,
    decoration: Option<&style::Snapshot>,
    allocate: impl FnOnce() -> std::io::Result<(std::fs::File, std::path::PathBuf)>,
) -> std::result::Result<std::path::PathBuf, String> {
    let _apartment = Apartment::new().map_err(|error| error.to_string())?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| error.to_string())?;
    let bitmap = decoration
        .copied()
        .unwrap_or_default()
        .compose(image, editor, &wic, opacity, crate::pixels::reserve)
        .map_err(|error| error.to_string())?;
    let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap).map_err(|error| error.to_string())?;
    drop(bitmap);
    // create_new atomically rejects existing files; never truncate targets first or delete existing user content on failure.
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
    // Field order releases WIC buffers before returning budgets.
    pub(super) bitmap: IWICBitmap,
    _lease: crate::byte_budget::Lease,
}

/// Filenames and encoding share committed crop dimensions; viewport zoom, pan, and window size do not participate.
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
    compose_frame(image, editor, wic, opacity, reserve, layout, None)
}
fn compose_frame(
    image: &Image,
    editor: &Editor,
    wic: &IWICImagingFactory,
    opacity: f32,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
    layout: Option<&source_resolution::Layout>,
    decoration: Option<&style::Snapshot>,
) -> Result<ComposedImage> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(Error::new(E_INVALIDARG, "Invalid base image opacity"));
    }
    // When Editor owns raster history, its committed snapshot is authoritative for export; old host image is compatibility input only.
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
    // Beyond the shared original-image budget, WIC output and D2D upload copies coexist; admit all before allocating.
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
        paint::Paint {
            target: &target,
            factory: &factory,
            bitmap: &bitmap,
            annotations: annotation_bitmap.as_ref(),
            brush: &brush,
            editor,
        }
        .draw(opacity, layout, decoration, (width, height), &reserve)?;
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

/// Both formats share one composited snapshot without redrawing; no file writes or system-clipboard access.
#[cfg(test)]
pub(super) fn clipboard_payload(
    image: &Image,
    editor: &Editor,
) -> Result<super::clipboard_bitmap::Payload> {
    clipboard_payload_with_opacity(image, editor, 1.0)
}

#[cfg(test)]
pub(super) fn clipboard_payload_with_opacity(
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> Result<super::clipboard_bitmap::Payload> {
    style::Snapshot::default().clipboard(image, editor, opacity)
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

/// Loom inline resources reuse committed-snapshot composition without extra clipboard DIBs; return actual cropped output dimensions.
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
    // Retain only a fixed write buffer, without recopying the entire PNG; encoders/composition bitmaps are already released.
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
