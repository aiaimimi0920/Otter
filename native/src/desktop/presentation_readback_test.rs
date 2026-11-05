//! 仅测试读实际GPU纹理；上限256²，staging/返回像素分别计费，不进入产品帧热路径。
use windows::Win32::Graphics::{Direct3D11::*, Dxgi::Common::*};
pub(super) struct Pixels {
    pub size: [u32; 2],
    pub bytes: crate::pixels::Pixels,
}
struct Mapped<'a>(&'a ID3D11DeviceContext, &'a ID3D11Texture2D);
impl Drop for Mapped<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0.Unmap(self.1, 0);
        }
    }
}
pub(super) fn texture(device: &ID3D11Device, source: &ID3D11Texture2D) -> Pixels {
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source.GetDesc(&mut desc);
        assert!((1..=256).contains(&desc.Width) && (1..=256).contains(&desc.Height));
        assert_eq!(desc.Format, DXGI_FORMAT_B8G8R8A8_UNORM);
        assert_eq!(desc.SampleDesc.Count, 1);
        let length = desc.Width as usize * desc.Height as usize * 4;
        let _staging =
            crate::pixels::reserve(((desc.Width as usize * 4 + 255) & !255) * desc.Height as usize)
                .unwrap();
        let output = crate::pixels::reserve(length).unwrap();
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let mut staging = None;
        device
            .CreateTexture2D(&desc, None, Some(&mut staging))
            .unwrap();
        let staging = staging.unwrap();
        let context = device.GetImmediateContext().unwrap();
        context.CopyResource(&staging, source);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
        let _mapped = Mapped(&context, &staging);
        let mut bytes = Vec::with_capacity(length);
        for y in 0..desc.Height {
            bytes.extend_from_slice(std::slice::from_raw_parts(
                (mapped.pData as *const u8).add(y as usize * mapped.RowPitch as usize),
                desc.Width as usize * 4,
            ));
        }
        Pixels {
            size: [desc.Width, desc.Height],
            bytes: crate::pixels::Pixels::from_reserved(bytes, output),
        }
    }
}
