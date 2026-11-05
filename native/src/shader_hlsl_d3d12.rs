//! 原生 D3D12 单帧：upload→SRV、RT→copy/readback；不经 11On12，不接管窗口设备。
use super::{compile, contract, *};
use std::{mem::ManuallyDrop, time::Duration};
use windows::{
    core::Interface,
    Win32::{
        Foundation::RECT,
        Graphics::{Direct3D::D3D_FEATURE_LEVEL_11_0, Direct3D12::*, Dxgi::Common::*},
    },
};
#[path = "shader_hlsl_d3d12_pipeline.rs"]
mod pipeline;
#[path = "shader_hlsl_d3d12_resource.rs"]
mod resource;
use resource::Resource;

pub(super) struct Device {
    device: ID3D12Device,
    queue: ID3D12CommandQueue,
    fence: ID3D12Fence,
}
impl Device {
    pub fn new() -> Result<Self> {
        let fail = |e| api(Stage::Device, e);
        unsafe {
            let mut device = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device).map_err(fail)?;
            let device: ID3D12Device = device.ok_or(Error::Unsupported)?;
            // 显式拒绝软件适配器冒充 D3D12 hardware；WARP 仅由独立选项进入 D3D11。
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
            Ok(Self {
                device,
                queue,
                fence,
            })
        }
    }
    pub fn prepare(
        self,
        request: &Request<'_>,
        program: &compile::Program,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<Readback> {
        check(stopped, deadline)?;
        let owner = Submission::new(self, request, program, stopped, deadline)?;
        // 两阶段复用同一队列/命令 owner：先确认上传并释放 upload，后建输出资源。
        let owner = owner.submit(1, stopped, deadline)?;
        let (readback, footprint) = owner.draw(request, stopped, deadline)?;
        Ok(Readback {
            readback,
            footprint,
            width: request.width,
            height: request.height,
        })
    }
}
// 只持有已确认完成的 readback，不借用 Request/源码/纹理，允许 worker 先释放私有输入。
pub(super) struct Readback {
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
        // 完成后除 readback 外的 GPU 图已释放，才分配 CPU 输出，避免 4K 峰值叠加。
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

// 未确认完成时 ManuallyDrop 整体保留，而非只保留 device 或漏归还某些像素 lease。
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
            let (root, pipeline) =
                pipeline::create(&device.device, program, request.textures.len())?;
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
            // 未写像素（discard/clip）同样必须有确定的透明黑内容。
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
        let mut completed = self.submit(2, stopped, deadline)?;
        let readback = completed.readback.take().ok_or(Error::Layout)?;
        // completed 析构先归还图与租约；readback 独立维持对应设备的 COM 生命周期。
        Ok((readback, footprint))
    }
    fn submit(self, value: u64, stopped: &impl Fn() -> bool, deadline: Instant) -> Result<Self> {
        check(stopped, deadline)?;
        let command = self.list.cast().map_err(|e| api(Stage::Submit, e))?;
        // Execute 后直到 fence 确认保留完整图；失败不能单独释放 target/upload/租约。
        let owner = ManuallyDrop::new(self);
        unsafe {
            owner.device.queue.ExecuteCommandLists(&[Some(command)]);
            owner
                .device
                .queue
                .Signal(&owner.device.fence, value)
                .map_err(|e| api(Stage::Submit, e))?;
        }
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
            std::thread::sleep(Duration::from_millis(1));
        }
        let owner = ManuallyDrop::into_inner(owner);
        unsafe { owner.device.device.GetDeviceRemovedReason() }
            .map_err(|e| api(Stage::Readback, e))?;
        Ok(owner)
    }
}
