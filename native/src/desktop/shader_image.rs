//! 按帧合同分流；旧 premultiplied Canvas 反预乘，新 HLSL straight 帧进入独立无损适配。
//! 只准备完整结果，不发布应用状态；调用者须在后台运行并另验 request/source stamp。
use crate::{byte_budget::Lease, image_decode::Image, pixels::Pixels, shader_angle::Frame};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
};

pub struct Prepared {
    pub image: Image,
    pub png: Pixels,
}
impl Prepared {
    /// 只构造本地正式输出批次，不伪造远端 lease/delivery，也不绕过提交方的请求/来源核验。
    pub fn into_batch(
        self,
    ) -> std::result::Result<super::loom_output_batch::Batch, super::loom_output_batch::Error> {
        let value = super::loom_image_value::ImageValue::from_png(self.image, self.png)
            .map_err(|_| super::loom_output_batch::Error::InvalidOutputs)?;
        super::loom_output_batch::Batch::local_image(value)
    }
}

/// 适用于旧 ShaderRenderer 的默认 premultipliedAlpha=true；不是通用 straight RGBA 转换。
/// 输入/双像素缓冲/WIC 副本/编码结果沿用共享预算；WIC 内部工作不能强制中断。
pub fn prepare(
    frame: impl Into<super::shader_invocation::Frame>,
    cancel: &AtomicBool,
) -> Result<Prepared> {
    let frame = match frame.into() {
        super::shader_invocation::Frame::Native(frame) => {
            return super::shader_native_image::prepare(frame, cancel)
        }
        super::shader_invocation::Frame::Legacy(frame) => frame,
    };
    prepare_with(
        frame,
        cancel,
        crate::pixels::reserve,
        super::export::png_straight_bgra,
    )
}

fn prepare_with(
    frame: Frame,
    cancel: &AtomicBool,
    reserve: impl Fn(usize) -> Option<Lease>,
    encode: impl FnOnce(u32, u32, &Pixels, &AtomicBool) -> Result<Pixels>,
) -> Result<Prepared> {
    let (image, straight) = convert(frame, &|| cancel.load(Ordering::Acquire), reserve)?;
    let png = encode(image.width, image.height, &straight, cancel)?;
    drop(straight);
    // WIC 返回后再验一次：迟到取消不得发布半个结果或成功 PNG。
    check(&|| cancel.load(Ordering::Acquire))?;
    Ok(Prepared { image, png })
}

fn convert(
    frame: Frame,
    stopped: &impl Fn() -> bool,
    reserve: impl Fn(usize) -> Option<Lease>,
) -> Result<(Image, Pixels)> {
    check(stopped)?;
    let count = crate::shader_worker_wire::pixel_count(frame.width, frame.height)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid shader image dimensions"))?;
    if count != frame.rgba.len() {
        return Err(Error::new(E_INVALIDARG, "Invalid shader image pixels"));
    }
    let (mut display, display_lease) = allocate(count, &reserve)?;
    let (mut straight, straight_lease) = allocate(count, &reserve)?;
    let stride = frame.width as usize * 4;
    for ((source, display), straight) in frame
        .rgba
        .chunks_exact(stride)
        .zip(display.chunks_exact_mut(stride))
        .zip(straight.chunks_exact_mut(stride))
    {
        check(stopped)?;
        for ((source, display), straight) in source
            .chunks_exact(4)
            .zip(display.chunks_exact_mut(4))
            .zip(straight.chunks_exact_mut(4))
        {
            let alpha = source[3];
            // PNG 反预乘后再预乘恰好等于 min(raw, alpha)，不是 raw 再乘一次 alpha。
            display.copy_from_slice(&[
                source[2].min(alpha),
                source[1].min(alpha),
                source[0].min(alpha),
                alpha,
            ]);
            let scale = if alpha == 0 {
                0.0
            } else {
                1.0 / (f32::from(alpha) * (1.0 / 255.0))
            };
            straight.copy_from_slice(&[
                unpremultiply(source[2], scale),
                unpremultiply(source[1], scale),
                unpremultiply(source[0], scale),
                alpha,
            ]);
        }
    }
    check(stopped)?;
    Ok((
        Image {
            width: frame.width,
            height: frame.height,
            pixels: Pixels::from_reserved(display, display_lease),
        },
        Pixels::from_reserved(straight, straight_lease),
    ))
}

fn unpremultiply(channel: u8, scale: f32) -> u8 {
    // 逐步 f32 归一化/倒数/乘法和 ties-to-even 已与真实 Canvas PNG 的完整 256² 矩阵精确对照。
    // 不可化简为 c*255/a 或整数四舍五入：半整数附近的舍入将改变。
    ((f32::from(channel) * (1.0 / 255.0)) * scale * 255.0)
        .round_ties_even()
        .clamp(0.0, 255.0) as u8
}

fn allocate(count: usize, reserve: &impl Fn(usize) -> Option<Lease>) -> Result<(Vec<u8>, Lease)> {
    let error = || Error::new(E_OUTOFMEMORY, "Shader image pixel budget exhausted");
    let mut lease = reserve(count).ok_or_else(error)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(count).map_err(|_| error())?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(error());
    }
    bytes.resize(count, 0);
    Ok((bytes, lease))
}

fn check(stopped: &impl Fn() -> bool) -> Result<()> {
    if stopped() {
        Err(Error::new(E_ABORT, "Shader image conversion cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "shader_image_tests.rs"]
pub(crate) mod tests;
