//! 默认呈现设备按 UI 线程弱共享；最后一个租约释放后不保留 GPU/COM 资源。
//! 只共享 D3D11 device，窗口交换链、D2D context、合成树和 Live 捕获设备不归此 owner。
use super::selection::Backend;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_FAIL, HMODULE},
        Graphics::{Direct3D::*, Direct3D11::*},
    },
};

pub(super) struct Device(ID3D11Device);
impl Device {
    pub(super) fn get(&self) -> &ID3D11Device {
        &self.0
    }
}
thread_local! {
    // Rc/Weak 限定到调用线程，不跨线程共享单线程 D2D 的资源 owner。
    static HARDWARE: RefCell<Weak<Device>> = const { RefCell::new(Weak::new()) };
    static SOFTWARE: RefCell<Weak<Device>> = const { RefCell::new(Weak::new()) };
}
pub(super) fn acquire(backend: Backend) -> Result<Rc<Device>> {
    let (slot, driver) = match backend {
        Backend::D3d11Hardware => (&HARDWARE, D3D_DRIVER_TYPE_HARDWARE),
        Backend::D3d11Warp => (&SOFTWARE, D3D_DRIVER_TYPE_WARP),
    };
    slot.with(|slot| {
        cached(
            slot,
            || unsafe { create_device(driver).map(Device) },
            |device| unsafe { device.0.GetDeviceRemovedReason().is_ok() },
        )
    })
}
fn cached<T, E>(
    slot: &RefCell<Weak<T>>,
    create: impl FnOnce() -> std::result::Result<T, E>,
    healthy: impl FnOnce(&T) -> bool,
) -> std::result::Result<Rc<T>, E> {
    // 不跨驱动/COM 调用持有 RefCell 借用；失败不发布新缓存，也不破坏仍被旧窗口持有的租约。
    let existing = slot.borrow().upgrade();
    if let Some(existing) = existing.filter(|device| healthy(device)) {
        return Ok(existing);
    }
    let created = Rc::new(create()?);
    *slot.borrow_mut() = Rc::downgrade(&created);
    Ok(created)
}
pub(super) unsafe fn create_device(driver: D3D_DRIVER_TYPE) -> Result<ID3D11Device> {
    let mut device = None;
    D3D11CreateDevice(
        None,
        driver,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        Some(&mut device),
        None,
        None,
    )?;
    device.ok_or_else(|| Error::new(E_FAIL, "D3D11 device missing"))
}
#[cfg(test)]
#[path = "composition_device_tests.rs"]
mod tests;
