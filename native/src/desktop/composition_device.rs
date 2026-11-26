//! Default presentation devices are weakly shared per UI thread; retain no GPU/COM resources after the final lease is released.
//! Share the D3D11 device and matching D2D device; this owner excludes window swap chains, D2D contexts, composition trees, and Live capture devices.
use super::selection::Backend;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_FAIL, HMODULE},
        Graphics::{
            Direct2D::*,
            Direct3D::*,
            Direct3D11::*,
            Dxgi::{IDXGIDevice, IDXGIDevice3},
        },
    },
};

pub(super) struct Device {
    device11: ID3D11Device,
    device2d: ID2D1Device,
}
impl Device {
    pub(super) fn get(&self) -> &ID3D11Device {
        &self.device11
    }
    pub(super) fn device2d(&self) -> &ID2D1Device {
        &self.device2d
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        // Window-private swapchains and D2D contexts drop before the last Rc<Device> lease;
        // flush immediate context state and trim DXGI driver caches when returning to zero GPU windows.
        unsafe {
            if let Ok(context) = self.device11.GetImmediateContext() {
                context.ClearState();
                context.Flush();
            }
            if let Ok(dxgi) = self.device11.cast::<IDXGIDevice3>() {
                dxgi.Trim();
            }
        }
    }
}
thread_local! {
    // Rc/Weak stay on the calling thread; do not share single-threaded D2D resource owners across threads.
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
            || unsafe {
                let device11 = create_device(driver)?;
                let factory: ID2D1Factory1 =
                    D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
                let dxgi: IDXGIDevice = device11.cast()?;
                let device2d = factory.CreateDevice(&dxgi)?;
                Ok(Device { device11, device2d })
            },
            |device| unsafe { device.device11.GetDeviceRemovedReason().is_ok() },
        )
    })
}
fn cached<T, E>(
    slot: &RefCell<Weak<T>>,
    create: impl FnOnce() -> std::result::Result<T, E>,
    healthy: impl FnOnce(&T) -> bool,
) -> std::result::Result<Rc<T>, E> {
    // Hold no RefCell borrows across driver/COM calls; failures publish no new cache and preserve leases still held by old windows.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "composition_device_tests.rs"]
mod tests;
