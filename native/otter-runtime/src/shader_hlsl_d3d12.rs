//! Native D3D12 single frames: upload to SRV, RT to copy/readback; no 11On12 or takeover of window devices.
use super::{compile, contract, *};
use std::{
    mem::ManuallyDrop,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{Arc, Mutex},
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HANDLE, RECT, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Graphics::{Direct3D::D3D_FEATURE_LEVEL_11_0, Direct3D12::*, Dxgi::Common::*},
        System::Threading::{CreateEventW, WaitForSingleObject},
    },
};
#[path = "shader_hlsl_d3d12_pipeline.rs"]
pub mod pipeline;
#[path = "shader_hlsl_d3d12_resource.rs"]
pub mod resource;
use resource::Resource;

type CachedPso = (
    usize,
    usize,
    usize,
    ID3D12RootSignature,
    ID3D12PipelineState,
);

#[derive(Clone)]
pub(super) struct Device {
    device: ID3D12Device,
    queue: ID3D12CommandQueue,
    fence: ID3D12Fence,
    fence_event: Arc<OwnedHandle>,
    // Cache compiled `(ID3D12RootSignature, ID3D12PipelineState)` across warm 32-frame HLSL sessions so uniform
    // slider updates do not re-invoke driver PSO compilation on every frame.
    pso_cache: Arc<Mutex<Option<CachedPso>>>,
    next_fence: std::sync::Arc<std::sync::atomic::AtomicU64>,
}
impl Device {
    pub fn new() -> Result<Self> {
        let fail = |e| api(Stage::Device, e);
        unsafe {
            let mut device = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device).map_err(fail)?;
            let device: ID3D12Device = device.ok_or(Error::Unsupported)?;
            // Explicitly reject software adapters posing as D3D12 hardware; WARP enters D3D11 only through a separate option.
            let factory: windows::Win32::Graphics::Dxgi::IDXGIFactory4 =
                windows::Win32::Graphics::Dxgi::CreateDXGIFactory1().map_err(fail)?;
            let adapter: windows::Win32::Graphics::Dxgi::IDXGIAdapter1 = factory
                .EnumAdapterByLuid(device.GetAdapterLuid())
                .map_err(fail)?;
            if adapter.GetDesc1().map_err(fail)?.Flags
                & windows::Win32::Graphics::Dxgi::DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32
                != 0
            {
                return Err(Error::Unsupported);
            }
            let mut support = D3D12_FEATURE_DATA_FORMAT_SUPPORT {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ..Default::default()
            };
            device
                .CheckFeatureSupport(
                    D3D12_FEATURE_FORMAT_SUPPORT,
                    &mut support as *mut _ as *mut _,
                    std::mem::size_of_val(&support) as u32,
                )
                .map_err(fail)?;
            let required = D3D12_FORMAT_SUPPORT1_TEXTURE2D
                | D3D12_FORMAT_SUPPORT1_SHADER_SAMPLE
                | D3D12_FORMAT_SUPPORT1_RENDER_TARGET;
            if support.Support1.0 & required.0 != required.0 {
                return Err(Error::Unsupported);
            }
            let queue = device
                .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    ..Default::default()
                })
                .map_err(fail)?;
            let fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE).map_err(fail)?;
            let raw_event = CreateEventW(None, false, false, None).map_err(fail)?;
            let fence_event = Arc::new(OwnedHandle::from_raw_handle(raw_event.0));
            Ok(Self {
                device,
                queue,
                fence,
                fence_event,
                pso_cache: Arc::new(Mutex::new(None)),
                next_fence: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
            })
        }
    }
    fn next_fence_value(&self) -> Result<u64> {
        let value = self
            .next_fence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if value == u64::MAX {
            Err(Error::Unsupported)
        } else {
            Ok(value)
        }
    }
    pub(super) fn prepare(
        &self,
        request: &Request<'_>,
        program: &compile::Program,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<Readback> {
        check(stopped, deadline)?;
        let upload_fence = self.next_fence_value()?;
        let owner = Submission::new(self.clone(), request, program, stopped, deadline)?;
        // Both phases reuse one queue/command owner: confirm upload and release upload resources before creating output resources.
        let owner = owner.submit(upload_fence, stopped, deadline)?;
        let (readback, footprint) = owner.draw(request, stopped, deadline)?;
        Ok(Readback {
            readback,
            footprint,
            width: request.width,
            height: request.height,
        })
    }
}
// Hold only confirmed-complete readback, without borrowing Request/source code/textures, allowing the worker to release private inputs first.
pub(crate) struct Readback {
    readback: Resource,
    footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    width: u32,
    height: u32,
}
impl Readback {
    pub fn finish(self, stopped: &impl Fn() -> bool, deadline: Instant) -> Result<Frame> {
        check(stopped, deadline)?;
        let Self {
            readback,
            footprint,
            width,
            height,
        } = self;
        // After completion, allocate CPU output only once the GPU resource graph except readback is released, avoiding overlapping 4K peaks.
        let (mut output, output_lease) = contract::allocate(contract::image_bytes(width, height)?)?;
        let mapped = resource::Mapping::new(&readback.value, true, readback.bytes)?;
        let stride = width as usize * 4;
        for y in 0..height as usize {
            check(stopped, deadline)?;
            let offset = footprint.Offset as usize + y * footprint.Footprint.RowPitch as usize;
            if offset
                .checked_add(stride)
                .filter(|n| *n <= readback.bytes)
                .is_none()
            {
                return Err(Error::Layout);
            }
            let source = unsafe {
                std::slice::from_raw_parts(mapped.pointer.cast::<u8>().add(offset), stride)
            };
            output[y * stride..(y + 1) * stride].copy_from_slice(source);
        }
        check(stopped, deadline)?;
        Ok(Frame {
            width,
            height,
            rgba: Pixels::from_reserved(output, output_lease),
            backend: SelectedBackend::D3d12,
        })
    }
}

// Without confirmed completion, retain the entire ManuallyDrop rather than only the device or neglecting to return some pixel leases.
struct Submission {
    list: ID3D12GraphicsCommandList,
    _allocator: ID3D12CommandAllocator,
    _pipeline: ID3D12PipelineState,
    _root: ID3D12RootSignature,
    _views: ID3D12DescriptorHeap,
    _rtv: ID3D12DescriptorHeap,
    _textures: Vec<Resource>,
    _uploads: Vec<Resource>,
    _constants: Resource,
    target: Option<Resource>,
    readback: Option<Resource>,
    device: Device,
}
impl Submission {
    fn new(
        device: Device,
        request: &Request<'_>,
        program: &compile::Program,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<Self> {
        let fail = |e| api(Stage::Resource, e);
        unsafe {
            let pso_key = (
                program.vertex.as_raw() as usize,
                program.pixel.as_raw() as usize,
                request.textures.len(),
            );
            let cached_pso = device.pso_cache.lock().ok().and_then(|guard| {
                guard.as_ref().and_then(|(v, p, count, root, pso)| {
                    ((*v, *p, *count) == pso_key).then(|| (root.clone(), pso.clone()))
                })
            });
            let (root, pipeline) = match cached_pso {
                Some(pair) => pair,
                None => {
                    let created =
                        pipeline::create(&device.device, program, request.textures.len())?;
                    if let Ok(mut guard) = device.pso_cache.lock() {
                        *guard = Some((
                            pso_key.0,
                            pso_key.1,
                            pso_key.2,
                            created.0.clone(),
                            created.1.clone(),
                        ));
                    }
                    created
                }
            };
            let allocator: ID3D12CommandAllocator = device
                .device
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
                .map_err(fail)?;
            let list: ID3D12GraphicsCommandList = device
                .device
                .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, &pipeline)
                .map_err(fail)?;
            let constants = contract::constants(request);
            let constant_buffer =
                Resource::buffer(&device.device, constants.len().div_ceil(256) * 256, false)?;
            constant_buffer.upload(&constants, None, stopped, deadline)?;
            let views: ID3D12DescriptorHeap = device
                .device
                .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                    Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                    NumDescriptors: request.textures.len() as u32,
                    Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                    NodeMask: 0,
                })
                .map_err(fail)?;
            let rtv: ID3D12DescriptorHeap = device
                .device
                .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                    Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                    NumDescriptors: 1,
                    ..Default::default()
                })
                .map_err(fail)?;
            let mut textures = Vec::with_capacity(request.textures.len());
            let mut uploads = Vec::with_capacity(request.textures.len());
            let increment = device
                .device
                .GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV)
                as usize;
            let mut descriptor = views.GetCPUDescriptorHandleForHeapStart();
            for texture in request.textures {
                check(stopped, deadline)?;
                let image =
                    Resource::texture(&device.device, texture.width, texture.height, false)?;
                let upload = Resource::buffer(&device.device, image.bytes, false)?;
                upload.upload(texture.rgba, Some(image.footprint), stopped, deadline)?;
                resource::copy(&list, &image.value, &upload.value, image.footprint, true);
                resource::transition(
                    &list,
                    &image.value,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                );
                device.device.CreateShaderResourceView(
                    &image.value,
                    Some(&D3D12_SHADER_RESOURCE_VIEW_DESC {
                        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                        ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                        Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                        Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D12_TEX2D_SRV {
                                MipLevels: 1,
                                ..Default::default()
                            },
                        },
                    }),
                    descriptor,
                );
                descriptor.ptr += increment;
                textures.push(image);
                uploads.push(upload);
            }
            list.Close().map_err(|e| api(Stage::Submit, e))?;
            Ok(Self {
                list,
                _allocator: allocator,
                _pipeline: pipeline,
                _root: root,
                _views: views,
                _rtv: rtv,
                _textures: textures,
                _uploads: uploads,
                _constants: constant_buffer,
                target: None,
                readback: None,
                device,
            })
        }
    }
    fn draw(
        mut self,
        request: &Request<'_>,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<(Resource, D3D12_PLACED_SUBRESOURCE_FOOTPRINT)> {
        self._uploads.clear();
        check(stopped, deadline)?;
        let target = Resource::texture(&self.device.device, request.width, request.height, true)?;
        let readback = Resource::buffer(&self.device.device, target.bytes, true)?;
        unsafe {
            self._allocator.Reset().map_err(|e| api(Stage::Submit, e))?;
            self.list
                .Reset(&self._allocator, &self._pipeline)
                .map_err(|e| api(Stage::Submit, e))?;
            let rtv_handle = self._rtv.GetCPUDescriptorHandleForHeapStart();
            self.device
                .device
                .CreateRenderTargetView(&target.value, None, rtv_handle);
            self.list.SetGraphicsRootSignature(&self._root);
            self.list.SetDescriptorHeaps(&[Some(self._views.clone())]);
            self.list.SetGraphicsRootDescriptorTable(
                0,
                self._views.GetGPUDescriptorHandleForHeapStart(),
            );
            self.list
                .SetGraphicsRootConstantBufferView(1, self._constants.value.GetGPUVirtualAddress());
            self.list
                .OMSetRenderTargets(1, Some(&rtv_handle), false, None);
            // Unwritten pixels (discard/clip) must also contain deterministic transparent black.
            self.list.ClearRenderTargetView(rtv_handle, &[0.; 4], None);
            self.list.RSSetViewports(&[D3D12_VIEWPORT {
                Width: request.width as f32,
                Height: request.height as f32,
                MinDepth: 0.,
                MaxDepth: 1.,
                ..Default::default()
            }]);
            self.list.RSSetScissorRects(&[RECT {
                left: 0,
                top: 0,
                right: request.width as i32,
                bottom: request.height as i32,
            }]);
            self.list.IASetPrimitiveTopology(
                windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            );
            self.list.DrawInstanced(3, 1, 0, 0);
            resource::transition(
                &self.list,
                &target.value,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_STATE_COPY_SOURCE,
            );
            resource::copy(
                &self.list,
                &target.value,
                &readback.value,
                target.footprint,
                false,
            );
            self.list.Close().map_err(|e| api(Stage::Submit, e))?;
        }
        let footprint = target.footprint;
        self.target = Some(target);
        self.readback = Some(readback);
        let draw_fence = self.device.next_fence_value()?;
        let mut completed = self.submit(draw_fence, stopped, deadline)?;
        let readback = completed.readback.take().ok_or(Error::Layout)?;
        // completed destruction first returns the graph and leases; readback independently maintains the corresponding device's COM lifetime.
        Ok((readback, footprint))
    }
    fn submit(self, value: u64, stopped: &impl Fn() -> bool, deadline: Instant) -> Result<Self> {
        check(stopped, deadline)?;
        let command = self.list.cast().map_err(|e| api(Stage::Submit, e))?;
        // Retain the complete graph from Execute until fence confirmation; failure must not independently release target/upload/leases.
        let owner = ManuallyDrop::new(self);
        unsafe {
            owner.device.queue.ExecuteCommandLists(&[Some(command)]);
            owner
                .device
                .queue
                .Signal(&owner.device.fence, value)
                .map_err(|e| api(Stage::Submit, e))?;
        }
        let mut attempts = 0u32;
        let mut registered = false;
        loop {
            check(stopped, deadline)?;
            let completed = unsafe { owner.device.fence.GetCompletedValue() };
            if completed == u64::MAX {
                return Err(unsafe { owner.device.device.GetDeviceRemovedReason() }
                    .err()
                    .map(|e| api(Stage::Readback, e))
                    .unwrap_or(Error::Unsupported));
            }
            if completed >= value {
                break;
            }
            if attempts < 8 {
                std::hint::spin_loop();
            } else if attempts < 16 {
                std::thread::yield_now();
            } else {
                // Wait on the D3D12 fence event instead of `thread::sleep(1ms)` (~15.6 ms OS timer granularity),
                // bounding each slice to 5 ms so `stopped` / `deadline` cancellation stays responsive.
                let event = HANDLE(owner.device.fence_event.as_raw_handle());
                // A timed-out wait does not cancel the outstanding fence notification.
                if !registered {
                    unsafe { owner.device.fence.SetEventOnCompletion(value, event) }
                        .map_err(|e| api(Stage::Readback, e))?;
                    registered = true;
                }
                let slice_ms = deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .min(5) as u32;
                let status = unsafe { WaitForSingleObject(event, slice_ms.max(1)) };
                if status != WAIT_OBJECT_0 && status != WAIT_TIMEOUT {
                    return Err(Error::TimedOut);
                }
            }
            attempts = attempts.saturating_add(1);
        }
        let owner = ManuallyDrop::into_inner(owner);
        unsafe { owner.device.device.GetDeviceRemovedReason() }
            .map_err(|e| api(Stage::Readback, e))?;
        Ok(owner)
    }
}
