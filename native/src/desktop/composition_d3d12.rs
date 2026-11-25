//! Window backend with D3D12 queues/swap chains; RenderSurface prefers it, with D2D drawing through the shared 11On12 queue.
#[path = "composition_d3d12_buffers.rs"]
mod buffers;
#[path = "composition_d3d12_device.rs"]
mod device;
#[cfg(test)]
#[path = "composition_d3d12_readback_test.rs"]
mod readback_test;
use super::{composition_lifetime as lifetime, composition_retirement as retirement};
pub use retirement::{Stage as RetirementStage, Status as RetirementStatus};
use std::{mem::ManuallyDrop, rc::Rc};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_FAIL, E_INVALIDARG, HWND, S_OK},
        Graphics::{
            Direct2D::{Common::*, *},
            DirectComposition::*,
            Dxgi::{Common::*, *},
        },
    },
};

pub struct CompositionD3d12(ManuallyDrop<State>);
struct State {
    context: ID2D1DeviceContext,
    buffers: Vec<buffers::Buffer>,
    chain: IDXGISwapChain3,
    visual: IDCompositionVisual,
    target: IDCompositionTarget,
    composition: IDCompositionDevice,
    opacity_effect: Option<IDCompositionEffectGroup>,
    opacity: f32,
    size: D2D_SIZE_U,
    ready: bool,
    failed: bool,
    present_trace: super::composition_present_status::Trace,
    retirement: Option<retirement::Retirement>,
    // Release the device last; views belong only to this window, without copying business documents or creating a second swap chain.
    device: Rc<device::Device>,
    _lease: lifetime::Lease,
}
impl CompositionD3d12 {
    #[cfg(test)]
    pub fn new(hwnd: HWND, size: D2D_SIZE_U) -> Result<Self> {
        Self::new_candidate(hwnd, size).map_err(super::composition_creation::Failure::into_error)
    }
    pub(super) fn new_candidate(
        hwnd: HWND,
        size: D2D_SIZE_U,
    ) -> super::composition_creation::Result<Self> {
        let owner = Self(ManuallyDrop::new(State::new(hwnd, size)?));
        owner.attach_with(|state| unsafe {
            state.visual.SetContent(&state.chain)?;
            state.target.SetRoot(&state.visual)?;
            state.composition.Commit()
        })
    }
    // Perform all association operations only after constructing the complete owner; failure evidence is not permission for safe destruction.
    // Return the original HRESULT here; starting asynchronous retirement does not authorize another candidate to take over the HWND.
    fn attach_with(
        mut self,
        attach: impl FnOnce(&mut State) -> Result<()>,
    ) -> super::composition_creation::Result<Self> {
        if let Err(error) = attach(&mut self.0) {
            self.retire_composition();
            return Err(super::composition_creation::Failure::attached(error));
        }
        Ok(self)
    }
    pub fn draw(&mut self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<()> {
        self.0.draw(paint)
    }
    pub fn present(&mut self) -> Result<bool> {
        self.0.present()
    }
    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        self.0.resize(size)
    }
    /// Only for creating same-device brushes/bitmaps and similar resources; draw exclusively controls frame boundaries.
    pub fn resource_target(&self) -> &ID2D1RenderTarget {
        &self.0.context
    }
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        self.0.set_opacity(opacity)
    }
    /// Report only backend-resource invalidation; arbitrary errors returned by business drawing closures are not device failures.
    pub fn requires_rebuild(&self) -> bool {
        self.0.retirement.is_some()
            || self.0.failed
            || self.0.buffers.len() != 2
            || self.0.device.ensure_usable().is_err()
    }
    /// One-way retirement: no further drawing/commits; Complete confirms DComp, not GPU drain.
    pub fn retire_composition(&mut self) -> RetirementStatus {
        self.0.retire_composition()
    }
}
impl State {
    fn retire_composition(&mut self) -> RetirementStatus {
        self.ready = false;
        self.failed = true;
        self.retirement
            .get_or_insert_with(|| {
                retirement::Retirement::begin(&self.composition, &self.visual, &self.target)
            })
            .poll()
    }
    fn new(hwnd: HWND, size: D2D_SIZE_U) -> Result<Self> {
        validate(size)?;
        let lease = lifetime::Lease::new()?;
        let device = device::acquire()?;
        unsafe {
            let dxgi: IDXGIDevice = device.device11.cast()?;
            let factory: IDXGIFactory2 = dxgi.GetAdapter()?.GetParent()?;
            let chain: IDXGISwapChain3 = factory
                .CreateSwapChainForComposition(
                    &device.queue,
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
                )?
                .cast()?;
            let context = device
                .device2d
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let buffers = buffers::create(&device, &chain, &context)?;
            let composition: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
            let target = composition.CreateTargetForHwnd(hwnd, true)?;
            let visual = composition.CreateVisual()?;
            Ok(Self {
                context,
                buffers,
                chain,
                visual,
                target,
                composition,
                opacity_effect: None,
                opacity: 1.0,
                size,
                ready: false,
                failed: false,
                present_trace: Default::default(),
                retirement: None,
                device,
                _lease: lease,
            })
        }
    }
    fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(Error::new(E_INVALIDARG, "Invalid window opacity"));
        }
        if self.failed {
            return Err(Error::new(E_FAIL, "D3D12 candidate requires rebuild"));
        }
        self.device.ensure_usable()?;
        if opacity == self.opacity {
            return Ok(());
        }
        // Composition opacity changes neither backbuffers, document pixels, nor exports; update the committed value only after Commit succeeds.
        let result = (|| unsafe {
            let effect = match &self.opacity_effect {
                Some(effect) => effect.clone(),
                None => self.composition.CreateEffectGroup()?,
            };
            effect.SetOpacity2(opacity)?;
            self.visual.SetEffect(&effect)?;
            self.composition.Commit()?;
            self.opacity_effect = Some(effect);
            self.opacity = opacity;
            Ok(())
        })();
        if result.is_err() {
            // Pending DComp properties may have changed; do not treat the old value as a reliable rollback and continue committing other operations.
            self.failed = true;
            self.ready = false;
        }
        result
    }
    pub fn draw(&mut self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<()> {
        self.ready = false;
        if self.failed {
            return Err(Error::new(E_FAIL, "D3D12 candidate requires rebuild"));
        }
        let index = unsafe { self.chain.GetCurrentBackBufferIndex() } as usize;
        let buffer = self
            .buffers
            .get(index)
            .ok_or_else(|| Error::new(E_FAIL, "D3D12 surface requires rebuild"))?;
        let frame = buffers::Frame::begin(&self.device, &self.context, buffer)?;
        let result = paint(&self.context);
        let ended = frame.end();
        self.failed = ended.is_err();
        result?;
        ended?;
        self.ready = true;
        Ok(())
    }
    pub fn present(&mut self) -> Result<bool> {
        self.ensure_attached()?;
        self.device.ensure_usable()?;
        if !std::mem::take(&mut self.ready) {
            return Err(Error::new(
                E_FAIL,
                "D3D12 present requires a completed draw",
            ));
        }
        let status = unsafe { self.chain.Present(0, DXGI_PRESENT::default()) };
        self.present_trace.observe("d3d12", status);
        self.failed = status.is_err();
        status.ok()?;
        Ok(status == S_OK)
    }
    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        self.ensure_attached()?;
        validate(size)?;
        self.device.ensure_usable()?;
        if size == self.size && self.buffers.len() == 2 {
            return Ok(());
        }
        self.ready = false;
        if let Err(error) = self.device.drain() {
            self.failed = true;
            return Err(error);
        }
        unsafe {
            self.context.SetTarget(None);
            self.buffers.clear();
            // Flush deferred wrapped-resource destruction before ResizeBuffers; retain an undrawable state on failure.
            self.device.immediate.ClearState();
            self.device.immediate.Flush();
            self.chain.ResizeBuffers(
                0,
                size.width,
                size.height,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG::default(),
            )?;
            self.buffers = buffers::create(&self.device, &self.chain, &self.context)?;
        }
        self.size = size;
        Ok(())
    }
    fn ensure_attached(&self) -> Result<()> {
        if self.retirement.is_some() {
            return Err(Error::new(E_FAIL, "D3D12 composition is retiring"));
        }
        Ok(())
    }
}
fn validate(size: D2D_SIZE_U) -> Result<()> {
    if size.width == 0
        || size.height == 0
        || size.width > 8192
        || size.height > 8192
        || u64::from(size.width) * u64::from(size.height) > 16_777_216
    {
        return Err(Error::new(
            E_INVALIDARG,
            "D3D12 surface exceeds presentation budget",
        ));
    }
    Ok(())
}
impl Drop for CompositionD3d12 {
    fn drop(&mut self) {
        unsafe {
            let state = &mut self.0;
            if let Some(retirement) = &mut state.retirement {
                if retirement.poll() != RetirementStatus::Complete {
                    // Preserve the whole GPU graph and lease, not just worker COM references.
                    // Existing lifetime budgets bound this retention; no additional wait or Commit.
                    eprintln!("native-d3d12-retirement-unconfirmed-retained");
                    return;
                }
            }
            state.context.SetTarget(None);
            if state.retirement.is_none() {
                let _ = state.visual.SetContent(None);
                let _ = state.target.SetRoot(None);
                let _ = state.composition.Commit();
            }
            if let Err(error) = state.device.drain() {
                if state.device.device12.GetDeviceRemovedReason().is_ok() {
                    // Unconfirmed completion: poison the shared generation, retain the complete window graph and leases, and reject further borrowing.
                    // Do not treat fence timeouts as normal closure or force RemoveDevice on a process-shared device.
                    state.device.poison();
                    eprintln!(
                        "native-d3d12-close-unconfirmed-retained: {:?}",
                        error.code()
                    );
                    return;
                }
                eprintln!("native-d3d12-close-device-removed: {:?}", error.code());
            }
            state.buffers.clear();
            state.device.immediate.ClearState();
            state.device.immediate.Flush();
            ManuallyDrop::drop(&mut self.0);
        }
    }
}

#[cfg(test)]
#[path = "composition_d3d12_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "composition_d3d12_creation_tests.rs"]
mod creation_tests;
