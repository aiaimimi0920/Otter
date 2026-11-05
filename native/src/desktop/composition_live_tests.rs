use super::*;
use crate::desktop::{Apartment, Window};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};
pub(super) unsafe fn frame(device: &ID3D11Device, color: [u8; 4]) -> Arc<Frame<Texture>> {
    sized_frame(device, color, 2, 2)
}
unsafe fn sized_frame(
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
unsafe fn pixel(surface: &Surface) -> [u8; 4] {
    let bytes = pixels(surface);
    let offset = (8 * surface.composition.state.size.width as usize + 8) * 4;
    bytes[offset..offset + 4].try_into().unwrap()
}
unsafe fn pixels(surface: &Surface) -> Vec<u8> {
    let device: ID3D11Device = surface.composition.state.chain.GetDevice().unwrap();
    let context = device.GetImmediateContext().unwrap();
    let back: ID3D11Texture2D = surface.composition.state.chain.GetBuffer(0).unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    back.GetDesc(&mut desc);
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .unwrap();
    let staging = staging.unwrap();
    context.CopyResource(&staging, &back);
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context
        .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        .unwrap();
    let row = desc.Width as usize * 4;
    let mut output = vec![0; row * desc.Height as usize];
    for y in 0..desc.Height as usize {
        std::ptr::copy_nonoverlapping(
            (mapped.pData as *const u8).add(y * mapped.RowPitch as usize),
            output.as_mut_ptr().add(y * row),
            row,
        );
    }
    context.Unmap(&staging, 0);
    output
}
#[test]
fn live_chrome_gpu_pixels_match_theme_and_never_pollute_source_snapshot() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                128,
                64,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let device = create_device(D3D_DRIVER_TYPE_WARP).unwrap();
        let protection: ID3D11Multithread = device.GetImmediateContext().unwrap().cast().unwrap();
        let _ = protection.SetMultithreadProtected(true);
        let frame = sized_frame(&device, [60, 50, 40, 255], 4, 2);
        let surface = Surface::new(
            window.0,
            D2D_SIZE_U {
                width: 128,
                height: 64,
            },
            frame.clone(),
        )
        .unwrap();
        surface.draw().unwrap();
        let bytes = pixels(&surface);
        let at = |x: usize, y: usize| &bytes[(y * 128 + x) * 4..(y * 128 + x + 1) * 4];
        assert_eq!(at(32, 0), &[94, 197, 34, 255]);
        assert_eq!(at(32, 1), &[94, 197, 34, 255]);
        assert_eq!(at(32, 2), &[60, 50, 40, 255]);
        assert_eq!(at(10, 0), &[56, 255, 217, 255]);
        assert_eq!(at(0, 10), &[56, 255, 217, 255]);
        assert_eq!(at(8, 8), &[60, 50, 40, 255]);
        surface.draw_chrome(false).unwrap();
        let clean = pixels(&surface);
        assert!(clean
            .chunks_exact(4)
            .all(|pixel| pixel == [60, 50, 40, 255]));
        surface.draw_chrome(true).unwrap();
        assert_eq!(pixels(&surface), bytes);
        let raw =
            crate::live_texture::readback::read(frame, &std::sync::atomic::AtomicBool::new(false))
                .unwrap();
        assert_eq!(&*raw.pixels, &[60, 50, 40, 255].repeat(8));
        if let Some(path) = std::env::var_os("OTTER_NATIVE_LIVE_CHROME_PNG") {
            let mut rgba = bytes;
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 128, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&rgba)
                .unwrap();
        }
    }
}
#[test]
fn hidden_live_surface_draws_shared_gpu_pixels_reuses_bitmap_and_retains_budget_owner() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let device = create_device(D3D_DRIVER_TYPE_WARP).unwrap();
        let protection: ID3D11Multithread = device.GetImmediateContext().unwrap().cast().unwrap();
        assert!(validate(&frame(&device, [0, 0, 255, 255]).resource).is_err());
        let _ = protection.SetMultithreadProtected(true);
        let red = frame(&device, [0, 0, 255, 255]);
        let weak = Arc::downgrade(&red);
        let mut surface = Surface::new(
            window.0,
            D2D_SIZE_U {
                width: 32,
                height: 32,
            },
            red.clone(),
        )
        .unwrap();
        let bitmap = surface.bitmap.as_raw();
        assert!(!surface.set_frame(red.clone()).unwrap());
        assert_eq!(surface.bitmap.as_raw(), bitmap);
        drop(red);
        surface.draw().unwrap();
        assert_eq!(pixel(&surface), [0, 0, 255, 255]);
        assert!(weak.upgrade().is_some());
        let other_device = create_device(D3D_DRIVER_TYPE_WARP).unwrap();
        let other_protection: ID3D11Multithread =
            other_device.GetImmediateContext().unwrap().cast().unwrap();
        let _ = other_protection.SetMultithreadProtected(true);
        assert!(surface
            .set_frame(frame(&other_device, [255, 0, 0, 255]))
            .is_err());
        assert_eq!(surface.bitmap.as_raw(), bitmap);
        let mut invalid = frame(&device, [0, 0, 255, 255]);
        Arc::get_mut(&mut invalid).unwrap().resource.format = DXGI_FORMAT_R16G16B16A16_FLOAT;
        assert!(surface.set_frame(invalid).is_err());
        assert_eq!(surface.bitmap.as_raw(), bitmap);
        surface.set_frame(frame(&device, [0, 128, 0, 128])).unwrap();
        assert!(weak.upgrade().is_none());
        surface
            .resize(D2D_SIZE_U {
                width: 16,
                height: 16,
            })
            .unwrap();
        surface.draw().unwrap();
        assert_eq!(pixel(&surface), [0, 128, 0, 128]);
        surface
            .set_frame(sized_frame(&device, [255, 0, 0, 255], 4, 2))
            .unwrap();
        surface.draw().unwrap();
        assert_eq!(pixel(&surface), [255, 0, 0, 255]);
        crate::desktop::composition::tests::retire(&mut surface.composition);
        assert!(surface.draw().is_err());
        assert!(surface.present().is_err());
    }
}
