//! 专属 child 内的 D3D11 hardware/WARP 单帧。无 HWND、交换链、全局设备或隐藏重试。
use super::{compile, contract, *};
use std::time::Duration;
use windows::Win32::{
    Foundation::HMODULE,
    Graphics::{
        Direct3D::*,
        Direct3D11::*,
        Dxgi::{Common::*, DXGI_ERROR_WAS_STILL_DRAWING},
    },
};

pub(super) struct Device {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    backend: SelectedBackend,
}
impl Device {
    pub fn new(warp: bool) -> Result<Self> {
        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(
                None,
                if warp {
                    D3D_DRIVER_TYPE_WARP
                } else {
                    D3D_DRIVER_TYPE_HARDWARE
                },
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        }
        .map_err(|e| api(Stage::Device, e))?;
        let device = device.ok_or(Error::Unsupported)?;
        let required = (D3D11_FORMAT_SUPPORT_TEXTURE2D.0
            | D3D11_FORMAT_SUPPORT_SHADER_SAMPLE.0
            | D3D11_FORMAT_SUPPORT_RENDER_TARGET.0) as u32;
        if unsafe { device.CheckFormatSupport(DXGI_FORMAT_R8G8B8A8_UNORM) }
            .map_err(|e| api(Stage::Device, e))?
            & required
            != required
        {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            device,
            context: context.ok_or(Error::Unsupported)?,
            backend: if warp {
                SelectedBackend::Warp
            } else {
                SelectedBackend::D3d11
            },
        })
    }
    pub fn render(
        self,
        request: &Request<'_>,
        program: &compile::Program,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<Frame> {
        check(stopped, deadline)?;
        let count = contract::image_bytes(request.width, request.height)?;
        // 明确纹理像素副本预算；COM/驱动开销另外由 Job 限制，并非总显存实测值。
        let gpu_bytes = request
            .textures
            .iter()
            .try_fold(count.checked_mul(2).ok_or(Error::Budget)?, |sum, t| {
                sum.checked_add(t.rgba.len()).ok_or(Error::Budget)
            })?;
        let _gpu_lease = crate::pixels::reserve(gpu_bytes).ok_or(Error::Budget)?;
        let _bindings = Bindings(&self.context);
        let (mut output, output_lease) = contract::allocate(count)?;
        let resource = |e| api(Stage::Resource, e);
        let pipeline = |e| api(Stage::Pipeline, e);
        unsafe {
            let target = self.texture(
                request.width,
                request.height,
                D3D11_USAGE_DEFAULT,
                D3D11_BIND_RENDER_TARGET.0 as u32,
                0,
                None,
            )?;
            let staging = self.texture(
                request.width,
                request.height,
                D3D11_USAGE_STAGING,
                0,
                D3D11_CPU_ACCESS_READ.0 as u32,
                None,
            )?;
            let mut rtv = None;
            self.device
                .CreateRenderTargetView(&target, None, Some(&mut rtv))
                .map_err(resource)?;
            let mut views = Vec::with_capacity(request.textures.len());
            for texture in request.textures {
                check(stopped, deadline)?;
                let image = self.texture(
                    texture.width,
                    texture.height,
                    D3D11_USAGE_IMMUTABLE,
                    D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    0,
                    Some(D3D11_SUBRESOURCE_DATA {
                        pSysMem: texture.rgba.as_ptr().cast(),
                        SysMemPitch: texture.width * 4,
                        SysMemSlicePitch: 0,
                    }),
                )?;
                let mut view = None;
                self.device
                    .CreateShaderResourceView(&image, None, Some(&mut view))
                    .map_err(resource)?;
                views.push(Some(view.ok_or(Error::Unsupported)?));
            }
            let mut vertex = None;
            let mut pixel = None;
            self.device
                .CreateVertexShader(compile::bytes(&program.vertex), None, Some(&mut vertex))
                .map_err(pipeline)?;
            self.device
                .CreatePixelShader(compile::bytes(&program.pixel), None, Some(&mut pixel))
                .map_err(pipeline)?;
            let constants = contract::constants(request);
            let mut buffer = None;
            self.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: constants.len() as u32,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: constants.as_ptr().cast(),
                        ..Default::default()
                    }),
                    Some(&mut buffer),
                )
                .map_err(resource)?;
            let mut sampler = None;
            self.device
                .CreateSamplerState(
                    &D3D11_SAMPLER_DESC {
                        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                        AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                        AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                        MaxAnisotropy: 1,
                        ComparisonFunc: D3D11_COMPARISON_NEVER,
                        MinLOD: 0.,
                        MaxLOD: f32::MAX,
                        ..Default::default()
                    },
                    Some(&mut sampler),
                )
                .map_err(pipeline)?;
            let mut raster = None;
            self.device
                .CreateRasterizerState(
                    &D3D11_RASTERIZER_DESC {
                        FillMode: D3D11_FILL_SOLID,
                        CullMode: D3D11_CULL_NONE,
                        DepthClipEnable: true.into(),
                        ..Default::default()
                    },
                    Some(&mut raster),
                )
                .map_err(pipeline)?;
            check(stopped, deadline)?;
            // discard/clip 不会写入目标；透明黑基线不能依赖驱动的初始内容。
            self.context
                .ClearRenderTargetView(rtv.as_ref().ok_or(Error::Unsupported)?, &[0.; 4]);
            self.context.RSSetState(raster.as_ref());
            self.context.VSSetShader(vertex.as_ref(), None);
            self.context.PSSetShader(pixel.as_ref(), None);
            self.context.PSSetConstantBuffers(0, Some(&[buffer]));
            self.context.PSSetShaderResources(0, Some(&views));
            self.context.PSSetSamplers(0, Some(&[sampler]));
            self.context.OMSetRenderTargets(Some(&[rtv]), None);
            self.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                Width: request.width as f32,
                Height: request.height as f32,
                MinDepth: 0.,
                MaxDepth: 1.,
                ..Default::default()
            }]));
            self.context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.context.Draw(3, 0);
            self.context.CopyResource(&staging, &target);
            self.context.Flush();
            let mapped = self.map(&staging, stopped, deadline)?;
            let _unmap = Mapped {
                context: &self.context,
                texture: &staging,
            };
            let stride = request.width as usize * 4;
            if mapped.pData.is_null() || (mapped.RowPitch as usize) < stride {
                return Err(Error::Layout);
            }
            for y in 0..request.height as usize {
                check(stopped, deadline)?;
                let source = std::slice::from_raw_parts(
                    mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
                    stride,
                );
                output[y * stride..(y + 1) * stride].copy_from_slice(source);
            }
            self.device
                .GetDeviceRemovedReason()
                .map_err(|e| api(Stage::Readback, e))?;
        }
        check(stopped, deadline)?;
        Ok(Frame {
            width: request.width,
            height: request.height,
            rgba: Pixels::from_reserved(output, output_lease),
            backend: self.backend,
        })
    }
    unsafe fn texture(
        &self,
        width: u32,
        height: u32,
        usage: D3D11_USAGE,
        bind: u32,
        access: u32,
        data: Option<D3D11_SUBRESOURCE_DATA>,
    ) -> Result<ID3D11Texture2D> {
        let mut texture = None;
        self.device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: width,
                    Height: height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: usage,
                    BindFlags: bind,
                    CPUAccessFlags: access,
                    ..Default::default()
                },
                data.as_ref().map(|d| d as *const _),
                Some(&mut texture),
            )
            .map_err(|e| api(Stage::Resource, e))?;
        texture.ok_or(Error::Unsupported)
    }
    unsafe fn map(
        &self,
        staging: &ID3D11Texture2D,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<D3D11_MAPPED_SUBRESOURCE> {
        loop {
            check(stopped, deadline)?;
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            match self.context.Map(
                staging,
                0,
                D3D11_MAP_READ,
                D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
                Some(&mut mapped),
            ) {
                Ok(()) => return Ok(mapped),
                Err(error) if error.code() == DXGI_ERROR_WAS_STILL_DRAWING => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                Err(error) => return Err(api(Stage::Readback, error)),
            }
        }
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.context.ClearState();
            self.context.Flush();
        }
    }
}
struct Mapped<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
}
impl Drop for Mapped<'_> {
    fn drop(&mut self) {
        unsafe {
            self.context.Unmap(self.texture, 0);
        }
    }
}
struct Bindings<'a>(&'a ID3D11DeviceContext);
impl Drop for Bindings<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0.ClearState();
            self.0.Flush();
        }
    }
}
