//! 后台显示合成直接返回共享 PBGRA 像素；复用导出绘制，避免 PNG 编解码往返。
use super::{export, image::Image, Apartment};
use crate::{editor::Editor, pixels::Pixels, viewport::checked_image_bytes};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
        Graphics::Imaging::{CLSID_WICImagingFactory, IWICImagingFactory},
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
    },
};

/// 调用方持有不可变已提交快照，并在工作线程调用；COM 资源不跨线程返回。
/// WIC/Direct2D 调用不可中断，取消在合成和读回边界检查，不交付部分像素。
pub fn compose(image: &Image, editor: &Editor, cancel: &AtomicBool) -> Result<Image> {
    compose_reserved(image, editor, cancel, crate::pixels::reserve)
}

fn compose_reserved(
    image: &Image,
    editor: &Editor,
    cancel: &AtomicBool,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<Image> {
    check_cancel(cancel)?;
    if editor.draft().is_some() {
        return Err(Error::new(
            E_INVALIDARG,
            "Display composition requires committed snapshot",
        ));
    }
    let base = editor.layers().map(|layers| Image {
        width: layers.size.0,
        height: layers.size.1,
        pixels: layers.base.clone(),
    });
    let image = base.as_ref().unwrap_or(image);
    if checked_image_bytes(image.width, image.height) != Some(image.pixels.len()) {
        return Err(Error::new(
            E_INVALIDARG,
            "Invalid display composition image",
        ));
    }
    // 没有可见编辑时直接共享权威底图，即使历史中仍存在已撤销操作也不重渲染。
    if editor.elements().is_empty()
        && editor.annotation_frame().is_none_or(|frame| {
            frame.x == 0.0
                && frame.y == 0.0
                && frame.width == image.width as f64
                && frame.height == image.height as f64
        })
        && editor
            .layers()
            .is_none_or(|layers| layers.annotations.is_none())
    {
        check_cancel(cancel)?;
        return Ok(image.clone());
    }
    let _apartment = Apartment::new()?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let output = export::compose_with_budget(image, editor, &wic, &reserve)?;
    check_cancel(cancel)?;
    let (mut width, mut height) = (0, 0);
    // 合成位图和输出缓冲均局部持有；读回结束后释放 COM 和 WIC 预算租约。
    unsafe {
        output.bitmap.GetSize(&mut width, &mut height)?;
    }
    let size = checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid composed dimensions"))?;
    let mut lease = reserve(size)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Display readback budget exhausted"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Display readback allocation failed"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Display readback capacity exceeds budget",
        ));
    }
    bytes.resize(size, 0);
    unsafe {
        output
            .bitmap
            .CopyPixels(std::ptr::null(), width * 4, &mut bytes)?;
    }
    check_cancel(cancel)?;
    Ok(Image {
        width,
        height,
        pixels: Pixels::from_reserved(bytes, lease),
    })
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Display composition cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "display_composition_tests.rs"]
mod tests;
