//! Window-private composition chains for transparent pins; default D3D11 devices are weakly shared per UI thread, without a background frame loop.
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
    backend: Option<selection::Backend>,
    // Field order releases window-private COM resources first; weak caches do not extend the last default-device lease.
    _shared_device: Option<std::rc::Rc<devices::Device>>,
    // The D2D factory is single-threaded; this owner cannot migrate across threads even if underlying COM interfaces are marked Send.
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
            // Successful device creation is not success of the entire HWND/DXGI/D2D/DComp/bind chain.
            // Release local resources before association; failures after attempted association retain a retirement owner without taking over documents.
            let surface =
                Self::with_device_owner(hwnd, size, device.get(), Some((device.clone(), backend)))
                    .map_err(|error| (selection::Stage::Presentation, error))?;
            Ok(surface)
        })?;
        if let Some(failure) = selected.fallback {
            // Fixed backend/stage/HRESULT only, without image, package-path, or user-data logging; emit only on fallback.
            eprintln!(
                "native-presentation-fallback: {:?} selected={}",
                failure,
                selected.backend.label()
            );
        }
        Ok(selected.value)
    }
    pub(super) fn new_warp_candidate(
        hwnd: HWND,
        size: D2D_SIZE_U,
    ) -> super::composition_creation::Result<Self> {
        validate_size(size)?;
        let device = devices::acquire(selection::Backend::D3d11Warp)?;
        Self::with_device_owner(
            hwnd,
            size,
            device.get(),
            Some((device.clone(), selection::Backend::D3d11Warp)),
        )
    }
    pub(super) fn is_warp(&self) -> bool {
        self.state.backend == Some(selection::Backend::D3d11Warp)
    }
    /// Live builds its presentation chain with the capture device; textures from other devices are not directly shareable resources.
    pub fn with_device(hwnd: HWND, size: D2D_SIZE_U, device: &ID3D11Device) -> Result<Self> {
        Self::with_device_owner(hwnd, size, device, None)
            .map_err(super::composition_creation::Failure::into_error)
    }
    fn with_device_owner(
        hwnd: HWND,
        size: D2D_SIZE_U,
        device: &ID3D11Device,
        shared: Option<(std::rc::Rc<devices::Device>, selection::Backend)>,
    ) -> super::composition_creation::Result<Self> {
        validate_size(size)?;
        let lease = lifetime::Lease::new()?;
        let (shared, backend) = match shared {
            Some((owner, backend)) => (Some(owner), Some(backend)),
            None => (None, None),
        };
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
            let context = match &shared {
                Some(owner) => owner
                    .device2d()
                    .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?,
                None => {
                    let d2d: ID2D1Factory1 =
                        D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
                    d2d.CreateDevice(&dxgi)?
                        .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?
                }
            };
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
                    backend,
                    _shared_device: shared,
                    _thread: std::marker::PhantomData,
                    retirement: None,
                    attached: false,
                    failed: std::cell::Cell::new(false),
                    present_trace: Default::default(),
                    _lease: lease,
                }),
            };
            // Prepare D2D binding first; failure here means no composition association has ever been submitted to DComp.
            result.bind()?;
            result.state.attached = true;
            let attached = (|| {
                result.state.visual.SetContent(&result.state.chain)?;
                result.state.target.SetRoot(&result.state.visual)?;
                result.state.device.Commit()
            })();
            if let Err(error) = attached {
                // Partial properties or committed graphs may exist; retain the complete owner rather than treating the original error as proof of no binding.
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
            // Unbind the D2D target and release every old backbuffer reference before ResizeBuffers.
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
    /// Whole-window opacity belongs to composition visuals and changes neither base-image/annotation pixels nor exports.
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
        // Commit on changes; static pins do not trigger another frame themselves. No busy retry loop for now.
        let status = unsafe { self.state.chain.Present(0, DXGI_PRESENT::default()) };
        self.state.present_trace.observe("d3d11", status);
        if let Err(error) = status.ok() {
            self.state.failed.set(true);
            return Err(error);
        }
        // Positive statuses such as DXGI_STATUS_OCCLUDED do not confirm a new-frame submission.
        Ok(status == windows::Win32::Foundation::S_OK)
    }
    /// One-way retirement; only Complete proves DComp unbinding finished, not that GPU resources were released.
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
                    // ManuallyDrop retains all fields and shared device/window-graph leases, not just worker references.
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
