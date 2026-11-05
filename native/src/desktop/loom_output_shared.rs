//! Loom RGBA 映射复制到有预算的原生像素；本地映射 RAII 不等于远端资源租约释放。
use super::image::Image;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, E_ABORT, E_INVALIDARG, E_OUTOFMEMORY, HANDLE},
        System::Memory::*,
    },
};

pub struct Descriptor {
    handle: String,
    width: u32,
    height: u32,
    size: usize,
}
impl Descriptor {
    pub fn parse(value: &Value) -> Result<Self> {
        let invalid = || Error::new(E_INVALIDARG, "Invalid Loom shared image descriptor");
        let handle = value["handle"]
            .as_str()
            .filter(|name| {
                name.starts_with("Loom_Buffer_")
                    && name.len() > 12
                    && name.len() <= 256
                    && !name.chars().any(char::is_control)
            })
            .ok_or_else(invalid)?;
        if value["kind"].as_str() != Some("shared_memory")
            || value["format"].as_str() != Some("rgba8")
        {
            return Err(invalid());
        }
        let dimension = |field: &str| {
            value[field]
                .as_u64()
                .and_then(|number| u32::try_from(number).ok())
                .ok_or_else(invalid)
        };
        let (width, height) = (dimension("width")?, dimension("height")?);
        let size = crate::viewport::checked_image_bytes(width, height).ok_or_else(invalid)?;
        if value["size"].as_u64() != Some(size as u64) {
            return Err(invalid());
        }
        Ok(Self {
            handle: handle.into(),
            width,
            height,
            size,
        })
    }
    /// 无论复制成功或失败，调用方都必须按原执行身份处理远端 release。
    pub fn handle(&self) -> &str {
        &self.handle
    }
}
struct Mapping {
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    handle: HANDLE,
}
impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(self.view);
            let _ = CloseHandle(self.handle);
        }
    }
}
impl Mapping {
    fn open(descriptor: &Descriptor) -> Result<Self> {
        let name: Vec<u16> = descriptor.handle.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let handle = OpenFileMappingW(FILE_MAP_READ.0, false, PCWSTR(name.as_ptr()))?;
            let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, descriptor.size);
            if view.Value.is_null() {
                let error = Error::from_win32();
                let _ = CloseHandle(handle);
                return Err(error);
            }
            Ok(Self { view, handle })
        }
    }
}

/// 对端须按 formal 租约保持映射内容不变直到 release；本函数不验证映射创建者身份。
pub fn read(descriptor: &Descriptor, cancel: &AtomicBool) -> Result<Image> {
    check_cancel(cancel)?;
    let mut lease = crate::pixels::reserve(descriptor.size)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Loom shared pixel budget exhausted"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(descriptor.size)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Loom shared pixel allocation failed"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Loom shared pixel capacity exceeds budget",
        ));
    }
    let mapping = Mapping::open(descriptor)?;
    bytes.resize(descriptor.size, 0);
    let source = mapping.view.Value.cast::<u8>();
    for (index, pixel) in bytes.chunks_exact_mut(4).enumerate() {
        if index % 16384 == 0 {
            check_cancel(cancel)?;
        }
        // 不创建跨进程内存的 Rust 共享引用；只在已成功映射的限定范围读取。
        let rgba = unsafe {
            std::array::from_fn::<_, 4, _>(|channel| {
                source.add(index * 4 + channel).read_volatile()
            })
        };
        let alpha = rgba[3] as u16;
        pixel[0] = ((rgba[2] as u16 * alpha + 127) / 255) as u8;
        pixel[1] = ((rgba[1] as u16 * alpha + 127) / 255) as u8;
        pixel[2] = ((rgba[0] as u16 * alpha + 127) / 255) as u8;
        pixel[3] = rgba[3];
    }
    check_cancel(cancel)?;
    Ok(Image {
        width: descriptor.width,
        height: descriptor.height,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    })
}
fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Loom shared output cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "loom_output_shared_tests.rs"]
mod tests;
