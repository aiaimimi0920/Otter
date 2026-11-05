//! Source测试专属真实WARP纹理，不捕获用户桌面；独立常量像素。
use crate::live_texture::Texture;
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{Direct3D::D3D_DRIVER_TYPE_WARP, Direct3D11::*, Dxgi::Common::*},
    },
};
pub(crate) fn texture(size: [u32; 2], pixel: [u8; 4]) -> Texture {
    unsafe {
        let (mut device, mut context) = (None, None);
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_WARP,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
        .unwrap();
        let device = device.unwrap();
        let context = context.unwrap();
        let protection: ID3D11Multithread = context.cast().unwrap();
        let _ = protection.SetMultithreadProtected(true);
        let bytes = pixel.repeat((size[0] * size[1]) as usize);
        let desc = D3D11_TEXTURE2D_DESC {
            Width: size[0],
            Height: size[1],
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            ..Default::default()
        };
        let data = D3D11_SUBRESOURCE_DATA {
            pSysMem: bytes.as_ptr().cast(),
            SysMemPitch: size[0] * 4,
            SysMemSlicePitch: bytes.len() as u32,
        };
        let mut texture = None;
        device
            .CreateTexture2D(&desc, Some(&data), Some(&mut texture))
            .unwrap();
        crate::live_texture::snapshot(
            &context,
            &texture.unwrap(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap()
    }
}
