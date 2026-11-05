//! D3D12 主设备与唯一 direct queue；11On12 仅承载共享 D2D 绘制，不创建第二条呈现链。
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::{Rc, Weak},
};
use windows::Win32::Graphics::{Direct2D::*, Dxgi::IDXGIDevice};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{CloseHandle, E_FAIL, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Graphics::{Direct3D::*, Direct3D11::*, Direct3D11on12::*, Direct3D12::*},
        System::Threading::{CreateEventW, WaitForSingleObject},
    },
};

thread_local! {
    static SHARED: RefCell<Weak<Device>> = const { RefCell::new(Weak::new()) };
}

pub(super) fn acquire() -> Result<Rc<Device>> {
    SHARED.with(|slot| {
        // 不跨驱动调用保留 RefCell 借用；最后窗口释放后缓存不延长设备寿命。
        let existing = slot.borrow().upgrade();
        if let Some(device) = existing.filter(|device| device.ensure_usable().is_ok()) {
            return Ok(device);
        }
        let device = Rc::new(Device::new()?);
        *slot.borrow_mut() = Rc::downgrade(&device);
        Ok(device)
    })
}

pub(super) struct Device {
    pub device2d: ID2D1Device,
    pub bridge: ID3D11On12Device,
    pub immediate: ID3D11DeviceContext,
    pub device11: ID3D11Device,
    pub queue: ID3D12CommandQueue,
    fence: ID3D12Fence,
    next: Cell<u64>,
    // fence 注册的 event 与完整设备 owner 同寿命，超时不能先关闭仍待通知的句柄。
    event: Event,
    pub device12: ID3D12Device,
    poisoned: Cell<bool>,
    drawing: Cell<bool>,
    _generation: super::lifetime::Lease,
    _thread: PhantomData<Rc<()>>,
}
impl Device {
    pub fn new() -> Result<Self> {
        let generation = super::lifetime::Lease::generation()?;
        unsafe {
            let mut device12: Option<ID3D12Device> = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device12)?;
            let device12 = device12.ok_or_else(|| Error::new(E_FAIL, "Missing D3D12 device"))?;
            let queue: ID3D12CommandQueue =
                device12.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    ..Default::default()
                })?;
            let mut device11 = None;
            let mut immediate = None;
            D3D11On12CreateDevice(
                &device12,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT.0,
                None,
                Some(&[Some(queue.cast()?)]),
                0,
                Some(&mut device11),
                Some(&mut immediate),
                None,
            )?;
            let device11 = device11.ok_or_else(|| Error::new(E_FAIL, "Missing 11On12 device"))?;
            let immediate =
                immediate.ok_or_else(|| Error::new(E_FAIL, "Missing 11On12 context"))?;
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dxgi: IDXGIDevice = device11.cast()?;
            Ok(Self {
                device2d: factory.CreateDevice(&dxgi)?,
                bridge: device11.cast()?,
                immediate,
                device11,
                queue,
                fence: device12.CreateFence(0, D3D12_FENCE_FLAG_NONE)?,
                next: Cell::new(1),
                event: Event(CreateEventW(None, false, false, None)?),
                device12,
                poisoned: Cell::new(false),
                drawing: Cell::new(false),
                _generation: generation,
                _thread: PhantomData,
            })
        }
    }
    /// 仅 resize/销毁等资源边界等待；没有每帧 CPU fence 等待或后台轮询线程。
    pub fn drain(&self) -> Result<()> {
        self.ensure_usable()?;
        if self.drawing.get() {
            return Err(Error::new(
                E_FAIL,
                "D3D12 queue is inside a draw transaction",
            ));
        }
        let result = self.drain_queue();
        if result.is_err() {
            self.poison();
        }
        result
    }
    pub fn poison(&self) {
        self.poisoned.set(true);
    }
    pub fn ensure_usable(&self) -> Result<()> {
        if self.poisoned.get() {
            return Err(Error::new(
                E_FAIL,
                "D3D12 shared generation requires rebuild",
            ));
        }
        let result = unsafe { self.device12.GetDeviceRemovedReason() };
        if result.is_err() {
            self.poison();
        }
        result
    }
    pub fn begin_frame(&self) -> Result<FrameLease<'_>> {
        self.ensure_usable()?;
        if self.drawing.replace(true) {
            return Err(Error::new(E_FAIL, "Nested D3D12 draw transaction"));
        }
        Ok(FrameLease(self))
    }
    fn drain_queue(&self) -> Result<()> {
        unsafe {
            self.immediate.Flush();
            self.device12.GetDeviceRemovedReason()?;
            let value = self.next.get();
            self.next.set(
                value
                    .checked_add(1)
                    .ok_or_else(|| Error::new(E_FAIL, "Fence exhausted"))?,
            );
            self.queue.Signal(&self.fence, value)?;
            if self.fence.GetCompletedValue() < value {
                self.fence.SetEventOnCompletion(value, self.event.0)?;
                match WaitForSingleObject(self.event.0, 5000) {
                    WAIT_OBJECT_0 => {}
                    WAIT_FAILED => return Err(Error::from_win32()),
                    WAIT_TIMEOUT => {
                        return Err(Error::new(
                            E_FAIL,
                            "D3D12 queue drain did not finish within 5 seconds",
                        ))
                    }
                    _ => return Err(Error::new(E_FAIL, "Unexpected D3D12 fence wait status")),
                }
            }
            self.device12.GetDeviceRemovedReason()?;
            if self.fence.GetCompletedValue() < value {
                return Err(Error::new(
                    E_FAIL,
                    "D3D12 fence wake did not confirm completion",
                ));
            }
            Ok(())
        }
    }
}
pub(super) struct FrameLease<'a>(&'a Device);
impl Drop for FrameLease<'_> {
    fn drop(&mut self) {
        self.0.drawing.set(false);
    }
}
struct Event(HANDLE);
impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
