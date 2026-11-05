//! 固定双缓冲的 11On12/D2D 视图；保持底层 D3D12 资源直到队列排空与所有包装视图释放。
use super::{device::Device, *};
use windows::Win32::Graphics::{Direct3D11::*, Direct3D11on12::*, Direct3D12::*};

pub(super) struct Buffer {
    pub bitmap: ID2D1Bitmap1,
    pub wrapped: ID3D11Resource,
    pub _resource: ID3D12Resource,
}
pub(super) fn create(
    device: &Device,
    chain: &IDXGISwapChain3,
    context: &ID2D1DeviceContext,
) -> Result<Vec<Buffer>> {
    let mut buffers = Vec::with_capacity(2);
    for index in 0..2 {
        unsafe {
            let resource: ID3D12Resource = chain.GetBuffer(index)?;
            let mut wrapped: Option<ID3D11Resource> = None;
            // 当前切片没有先行 D3D12 原生绘制，输入与输出均为 PRESENT。
            // 11On12 负责实际 D2D 使用时的状态转换；后续原生 Shader 必须显式协商输入状态。
            device.bridge.CreateWrappedResource(
                &resource,
                &D3D11_RESOURCE_FLAGS {
                    BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                    ..Default::default()
                },
                D3D12_RESOURCE_STATE_PRESENT,
                D3D12_RESOURCE_STATE_PRESENT,
                &mut wrapped,
            )?;
            let wrapped =
                wrapped.ok_or_else(|| Error::new(E_FAIL, "Missing wrapped backbuffer"))?;
            let surface: IDXGISurface = wrapped.cast()?;
            let bitmap = context.CreateBitmapFromDxgiSurface(
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
            buffers.push(Buffer {
                bitmap,
                wrapped,
                _resource: resource,
            });
        }
    }
    Ok(buffers)
}

/// panic/错误同样解除 target 和包装资源，不能让下一帧继承 acquired 状态。
pub(super) struct Frame<'a> {
    device: &'a Device,
    context: &'a ID2D1DeviceContext,
    buffer: &'a Buffer,
    drawing: bool,
    _lease: super::device::FrameLease<'a>,
}
impl<'a> Frame<'a> {
    pub fn begin(
        device: &'a Device,
        context: &'a ID2D1DeviceContext,
        buffer: &'a Buffer,
    ) -> Result<Self> {
        let lease = device.begin_frame()?;
        unsafe {
            device
                .bridge
                .AcquireWrappedResources(&[Some(buffer.wrapped.clone())]);
            context.SetTarget(&buffer.bitmap);
            context.BeginDraw();
        }
        Ok(Self {
            device,
            context,
            buffer,
            drawing: true,
            _lease: lease,
        })
    }
    pub fn end(mut self) -> Result<()> {
        self.drawing = false;
        unsafe { self.context.EndDraw(None, None) }
    }
}
impl Drop for Frame<'_> {
    fn drop(&mut self) {
        unsafe {
            if self.drawing {
                let _ = self.context.EndDraw(None, None);
            }
            self.context.SetTarget(None);
            self.device
                .bridge
                .ReleaseWrappedResources(&[Some(self.buffer.wrapped.clone())]);
            self.device.immediate.Flush();
        }
    }
}
