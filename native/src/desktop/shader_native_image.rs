//! HLSL straight RGBA 的显示与正式 PNG 分流；绝不经旧 Canvas 反预乘还原原通道。
use crate::{image_decode::Image, pixels::Pixels, shader_hlsl::Frame};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_FAIL, E_OUTOFMEMORY},
};

pub fn prepare(frame: Frame, cancel: &AtomicBool) -> Result<super::shader_image::Prepared> {
    let stopped = || cancel.load(Ordering::Acquire);
    let deadline = Instant::now() + Duration::from_secs(15);
    let display = frame.pbgra(&stopped, deadline).map_err(|error| {
        Error::new(
            if stopped() { E_ABORT } else { E_FAIL },
            format!("HLSL display conversion: {error:?}"),
        )
    })?;
    let count = frame.rgba.len();
    let memory = || Error::new(E_OUTOFMEMORY, "HLSL PNG pixel budget exhausted");
    let mut lease = crate::pixels::reserve(count).ok_or_else(memory)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(count).map_err(|_| memory())?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(memory());
    }
    for row in frame.rgba.chunks_exact(frame.width as usize * 4) {
        if stopped() || Instant::now() >= deadline {
            return Err(Error::new(E_ABORT, "HLSL PNG conversion stopped"));
        }
        for pixel in row.chunks_exact(4) {
            bytes.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    let straight = Pixels::from_reserved(bytes, lease);
    let png = super::export::png_straight_bgra(frame.width, frame.height, &straight, cancel)?;
    if stopped() || Instant::now() >= deadline {
        return Err(Error::new(E_ABORT, "HLSL PNG conversion stopped"));
    }
    Ok(super::shader_image::Prepared {
        image: Image {
            width: frame.width,
            height: frame.height,
            pixels: display,
        },
        png,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_png_keeps_straight_channels_including_transparent_rgb() {
        let _apartment = super::super::Apartment::new().unwrap();
        let rgba = vec![255, 128, 64, 128, 16, 255, 77, 0, 1, 2, 3, 255];
        let frame = Frame {
            width: 3,
            height: 1,
            rgba: rgba.clone().into(),
            backend: crate::shader_hlsl::SelectedBackend::D3d12,
        };
        let prepared = prepare(frame, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            &*prepared.image.pixels,
            &[32, 64, 128, 128, 0, 0, 0, 0, 3, 2, 1, 255]
        );
        let decoded = crate::image_decode::shader::png(&prepared.png, 1024, &|| false).unwrap();
        assert_eq!(&*decoded.rgba, rgba.as_slice());
    }
    #[test]
    fn cancelled_native_frame_never_returns_a_formal_result() {
        let frame = Frame {
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 4].into(),
            backend: crate::shader_hlsl::SelectedBackend::D3d11,
        };
        assert!(prepare(frame, &AtomicBool::new(true)).is_err());
    }
}
