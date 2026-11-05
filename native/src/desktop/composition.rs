//! 透明钉图的窗口私有合成链；默认 D3D11 设备按 UI 线程弱共享，无后台帧循环。
use super::{composition_lifetime as lifetime, composition_retirement as retirement};
pub use retirement::{Stage as RetirementStage, Status as RetirementStatus};
use std::mem::ManuallyDrop;
#[path = "composition_device.rs"]
mod devices;
#[cfg(test)]
#[path = "composition_readback_test.rs"]
mod readback_test;
#[path = "composition_selection.rs"]
mod selection;
#[cfg(test)]
use devices::create_device;
#[cfg(test)]
use windows::Win32::Graphics::Direct3D::*;
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_FAIL, HWND},
        Graphics::{
            Direct2D::{Common::*, *},
            Direct3D11::*,
            DirectComposition::*,
            Dxgi::{Common::*, *},
        },
    },
};

pub struct Composition {
    state: ManuallyDrop<State>,
}
struct State {
    context: ID2D1DeviceContext,
    backing: Option<ID2D1Bitmap1>,
    chain: IDXGISwapChain1,
    opacity_effect: Option<IDCompositionEffectGroup>,
    opacity: f32,
    visual: IDCompositionVisual,
    target: IDCompositionTarget,
    device: IDCompositionDevice,
    size: D2D_SIZE_U,
    // 字段顺序使窗口私有 COM 资源先释放；弱缓存不延长最后一个默认设备租约。
    _shared_device: Option<std::rc::Rc<devices::Device>>,
    // D2D factory 为单线程；即使底层 COM 接口标记可发送，也不能跨线程迁移此 owner。
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
    retirement: Option<retirement::Retirement>,
    attached: bool,
    failed: std::cell::Cell<bool>,
    present_trace: super::composition_present_status::Trace,
    _lease: lifetime::Lease,
}
#[path = "composition_live.rs"]
pub mod live;
#[path = "composition_relay.rs"]
pub mod relay;

impl Composition {
    pub fn new(hwnd: HWND, size: D2D_SIZE_U) -> Result<Self> {
        Self::new_candidate(hwnd, size).map_err(super::composition_creation::Failure::into_error)
    }
    pub(super) fn new_candidate(
        hwnd: HWND,
        size: D2D_SIZE_U,
    ) -> super::composition_creation::Result<Self> {
        validate_size(size)?;
        let selected = selection::select(|backend| {
            let device = devices::acquire(backend)
                .map_err(|error| (selection::Stage::Device, error.into()))?;
            // 成功创建设备不等于 HWND/DXGI/D2D/DComp/bind 全链成功。
            // 未关联前释放局部资源；尝试关联后的失败由退休 owner 保留，不接管文档。
            let surface = Self::with_device_owner(hwnd, size, device.get(), Some(device.clone()))
                .map_err(|error| (selection::Stage::Presentation, error))?;
            Ok(surface)
        })?;
        if let Some(failure) = selected.fallback {
            // 固定后端/阶段/HRESULT，不记录图片、包路径或用户数据；仅降级时输出。
            eprintln!(
                "native-presentation-fallback: {:?} selected={}",
                failure,
                selected.backend.label()
            );
        }
        Ok(selected.value)
    }
    /// Live 使用捕获设备建立呈现链，不把异设备纹理当作可直接共享的资源。
    pub fn with_device(hwnd: HWND, size: D2D_SIZE_U, device: &ID3D11Device) -> Result<Self> {
        Self::with_device_owner(hwnd, size, device, None)
            .map_err(super::composition_creation::Failure::into_error)
    }
    fn with_device_owner(
        hwnd: HWND,
        size: D2D_SIZE_U,
        device: &ID3D11Device,
        shared: Option<std::rc::Rc<devices::Device>>,
    ) -> super::composition_creation::Result<Self> {
        validate_size(size)?;
        let lease = lifetime::Lease::new()?;
        unsafe {
            let dxgi: IDXGIDevice = device.cast()?;
            let factory: IDXGIFactory2 = dxgi.GetAdapter()?.GetParent()?;
            let chain = factory.CreateSwapChainForComposition(
                device,
                &DXGI_SWAP_CHAIN_DESC1 {
                    Width: size.width,
                    Height: size.height,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_STRETCH,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                    AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                    ..Default::default()
                },
                None,
            )?;
            let d2d: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let context = d2d
                .CreateDevice(&dxgi)?
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let composition: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
            let target = composition.CreateTargetForHwnd(hwnd, true)?;
            let visual = composition.CreateVisual()?;
            let mut result = Self {
                state: ManuallyDrop::new(State {
                    context,
                    backing: None,
                    chain,
                    opacity_effect: None,
                    opacity: 1.0,
                    visual,
                    target,
                    device: composition,
                    size,
                    _shared_device: shared,
                    _thread: std::marker::PhantomData,
                    retirement: None,
                    attached: false,
                    failed: std::cell::Cell::new(false),
                    present_trace: Default::default(),
                    _lease: lease,
                }),
            };
            // 先把 D2D 绑定准备好；这个阶段失败时从未向 DComp 提交合成关联。
            result.bind()?;
            result.state.attached = true;
            let attached = (|| {
                result.state.visual.SetContent(&result.state.chain)?;
                result.state.target.SetRoot(&result.state.visual)?;
                result.state.device.Commit()
            })();
            if let Err(error) = attached {
                // 可能存在部分属性或已提交图；保留完整 owner，不能把原错误当作未绑定证明。
                result.retire_composition();
                return Err(super::composition_creation::Failure::attached(error));
            }
            Ok(result)
        }
    }

    unsafe fn bind(&mut self) -> Result<()> {
        let surface: IDXGISurface = self.state.chain.GetBuffer(0)?;
        let bitmap = self.state.context.CreateBitmapFromDxgiSurface(
            &surface,
            Some(&D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                ..Default::default()
            }),
        )?;
        self.state.context.SetTarget(&bitmap);
        self.state.backing = Some(bitmap);
        Ok(())
    }

    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        self.ensure_attached()?;
        validate_size(size)?;
        if size == self.state.size {
            return Ok(());
        }
        unsafe {
            // ResizeBuffers 前必须解除 D2D target 并释放所有旧 backbuffer 引用。
            self.state.context.SetTarget(None);
            self.state.backing = None;
            self.state.failed.set(true);
            self.state.chain.ResizeBuffers(
                0,
                size.width,
                size.height,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG::default(),
            )?;
            self.bind()?;
            self.state.size = size;
            self.state.failed.set(false);
            Ok(())
        }
    }

    pub fn renderer(&self) -> &ID2D1RenderTarget {
        &self.state.context
    }
    pub(super) fn requires_rebuild(&self) -> bool {
        self.state.failed.get() || self.state.retirement.is_some() || self.state.backing.is_none()
    }
    pub(super) fn draw(&self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<()> {
        self.ensure_attached()?;
        let target = self.renderer();
        super::render_frame::draw_observed(target, || paint(target), || self.state.failed.set(true))
    }
    #[cfg(test)]
    pub(super) fn opacity_for_test(&self) -> f32 {
        self.state.opacity
    }
    /// 窗口整体透明度属于合成视觉，不改动底图/标注像素或导出内容。
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        self.ensure_attached()?;
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Invalid window opacity",
            ));
        }
        if opacity == self.state.opacity {
            return Ok(());
        }
        let result = (|| unsafe {
            let effect = match &self.state.opacity_effect {
                Some(effect) => effect.clone(),
                None => self.state.device.CreateEffectGroup()?,
            };
            effect.SetOpacity2(opacity)?;
            self.state.visual.SetEffect(&effect)?;
            self.state.device.Commit()?;
            self.state.opacity_effect = Some(effect);
            self.state.opacity = opacity;
            Ok(())
        })();
        if result.is_err() {
            self.state.failed.set(true);
        }
        result
    }

    pub fn present(&self) -> Result<()> {
        self.present_confirmed().map(|_| ())
    }
    pub(super) fn present_confirmed(&self) -> Result<bool> {
        self.ensure_attached()?;
        // 按变化提交；静态钉图不会自行触发下一帧。暂不使用忙重试循环。
        let status = unsafe { self.state.chain.Present(0, DXGI_PRESENT::default()) };
        self.state.present_trace.observe("d3d11", status);
        if let Err(error) = status.ok() {
            self.state.failed.set(true);
            return Err(error);
        }
        // 正的 DXGI_STATUS_OCCLUDED 等状态不是一次确定的新帧提交。
        Ok(status == windows::Win32::Foundation::S_OK)
    }
    /// 单向退休；只有 Complete 可作为 DComp 解绑完成凭证，不代表 GPU 已释放。
    pub fn retire_composition(&mut self) -> RetirementStatus {
        let state = &mut *self.state;
        state
            .retirement
            .get_or_insert_with(|| {
                retirement::Retirement::begin(&state.device, &state.visual, &state.target)
            })
            .poll()
    }
    pub(super) fn ensure_attached(&self) -> Result<()> {
        if self.requires_rebuild() || !self.state.attached {
            return Err(Error::new(
                E_FAIL,
                "D3D11 composition is retiring or not attached",
            ));
        }
        Ok(())
    }
}

fn validate_size(size: D2D_SIZE_U) -> Result<()> {
    if size.width == 0
        || size.height == 0
        || size.width > 8192
        || size.height > 8192
        || u64::from(size.width) * u64::from(size.height) > 16_777_216
    {
        return Err(Error::new(
            windows::Win32::Foundation::E_INVALIDARG,
            "Pin surface exceeds presentation budget",
        ));
    }
    Ok(())
}

impl Drop for Composition {
    fn drop(&mut self) {
        unsafe {
            let state = &mut self.state;
            if let Some(retirement) = &mut state.retirement {
                if retirement.poll() != RetirementStatus::Complete {
                    // ManuallyDrop 保留全部字段和共享设备/窗口图租约，而不是只保留 worker 引用。
                    eprintln!("native-d3d11-retirement-unconfirmed-retained");
                    return;
                }
            }
            state.context.SetTarget(None);
            state.backing = None;
            if state.attached && state.retirement.is_none() {
                let _ = state.visual.SetContent(None);
                let _ = state.target.SetRoot(None);
                let _ = state.device.Commit();
            }
            ManuallyDrop::drop(&mut self.state);
        }
    }
}

#[cfg(test)]
#[path = "composition_retirement_owner_tests.rs"]
mod retirement_owner_tests;
#[cfg(test)]
#[path = "composition_sharing_tests.rs"]
mod sharing_tests;
#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
