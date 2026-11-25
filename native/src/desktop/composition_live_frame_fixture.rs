//! Shared D3D frame fixture for the default and scoped desktop tests.
use super::*;

pub(super) unsafe fn frame(device: &ID3D11Device, color: [u8; 4]) -> Arc<Frame<Texture>> {
    sized_frame(device, color, 2, 2)
}
pub(super) unsafe fn sized_frame(
    device: &ID3D11Device,
    color: [u8; 4],
    width: u32,
    height: u32,
) -> Arc<Frame<Texture>> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
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
    let data = vec![color; (width * height) as usize];
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: data.as_ptr().cast(),
        SysMemPitch: width * 4,
        SysMemSlicePitch: width * height * 4,
    };
    let mut texture = None;
    device
        .CreateTexture2D(&desc, Some(&initial), Some(&mut texture))
        .unwrap();
    let texture = crate::live_texture::snapshot(
        &device.GetImmediateContext().unwrap(),
        &texture.unwrap(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    let source = crate::live_frames::Source::new();
    let mut consumer = source.subscribe().unwrap();
    source.publish(texture).unwrap();
    consumer.try_next().unwrap().unwrap().frame
}
