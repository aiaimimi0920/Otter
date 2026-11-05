use super::*;
use windows::Win32::{Foundation::HMODULE, Graphics::Direct3D::D3D_DRIVER_TYPE_WARP};
#[test]
fn bound_window_crop_copies_exact_bgra_pixels_and_uses_same_input_offset() {
    use crate::live_window_target::region::Crop;
    use windows::Win32::Foundation::RECT;
    unsafe {
        let (context, original) = fixture();
        let protection: ID3D11Multithread = context.cast().unwrap();
        let _ = protection.SetMultithreadProtected(true);
        let device = original.GetDevice().unwrap();
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        original.GetDesc(&mut desc);
        desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        let pixels: [u8; 16] = [
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        let mut texture = None;
        device
            .CreateTexture2D(
                &desc,
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: pixels.as_ptr().cast(),
                    SysMemPitch: 8,
                    SysMemSlicePitch: 16,
                }),
                Some(&mut texture),
            )
            .unwrap();
        let texture = texture.unwrap();
        let frame = RECT {
            left: -200,
            top: -100,
            right: -198,
            bottom: -98,
        };
        let crop = Crop::from_screen(
            frame,
            RECT {
                left: -199,
                top: -100,
                right: -198,
                bottom: -98,
            },
        )
        .unwrap();
        assert!(crop.matches_texture(frame, [2, 2]).unwrap());
        let copied =
            snapshot_region(&context, &texture, crop.pixels(), &AtomicBool::new(false)).unwrap();
        let source = crate::live_frames::Source::new();
        let mut consumer = source.subscribe().unwrap();
        source.publish(copied).unwrap();
        let prepared = consumer.try_next().unwrap().unwrap().frame;
        // 源池复用不能污染裁剪后的独立快照。
        context.UpdateSubresource(&texture, 0, None, [0u8; 16].as_ptr().cast(), 8, 16);
        let read = crate::live_texture::readback::read(prepared, &AtomicBool::new(false)).unwrap();
        assert_eq!((read.width, read.height), (1, 2));
        assert_eq!(&*read.pixels, &[40, 50, 60, 255, 100, 110, 120, 255]);
        assert_eq!((crop.input().left, crop.input().top), (1, 0));
        assert!(crop
            .matches_texture(
                RECT {
                    right: -199,
                    ..frame
                },
                [1, 2]
            )
            .is_err());
    }
}
#[test]
fn gpu_region_copy_preserves_exact_pixels_and_rejects_overflow_or_clipping() {
    unsafe {
        let (context, source) = fixture();
        let values: Vec<u16> = [0x0000, 0x3800, 0x3c00, 0x4000]
            .into_iter()
            .flat_map(|value| [value; 4])
            .collect();
        context.UpdateSubresource(&source, 0, None, values.as_ptr().cast(), 16, 32);
        let cancel = AtomicBool::new(false);
        let region = crate::capture_readback::Region {
            left: 1,
            top: 0,
            width: 1,
            height: 2,
        };
        let copied = snapshot_region(&context, &source, region, &cancel).unwrap();
        assert_eq!((copied.width, copied.height), (1, 2));
        let preview =
            crate::capture_readback::with_frame(&context, copied.texture(), &cancel, |value| {
                value.to_sdr(80.0, 1.0, &cancel)
            })
            .unwrap();
        assert_eq!(&*preview.pixels, &[188, 188, 188, 255, 255, 255, 255, 255]);
        for bad in [
            crate::capture_readback::Region {
                left: u32::MAX,
                ..region
            },
            crate::capture_readback::Region { left: 2, ..region },
            crate::capture_readback::Region { width: 0, ..region },
            crate::capture_readback::Region { top: 1, ..region },
        ] {
            assert!(matches!(
                snapshot_region(&context, &source, bad, &cancel),
                Err(Error::Dimensions)
            ));
        }
    }
}
unsafe fn fixture() -> (ID3D11DeviceContext, ID3D11Texture2D) {
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
    let desc = D3D11_TEXTURE2D_DESC {
        Width: 2,
        Height: 2,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        ..Default::default()
    };
    let pixels = [0x3c00u16; 16];
    let data = D3D11_SUBRESOURCE_DATA {
        pSysMem: pixels.as_ptr().cast(),
        SysMemPitch: 16,
        SysMemSlicePitch: 32,
    };
    let mut texture = None;
    device
        .unwrap()
        .CreateTexture2D(&desc, Some(&data), Some(&mut texture))
        .unwrap();
    (context.unwrap(), texture.unwrap())
}
#[test]
fn gpu_snapshot_survives_pool_texture_reuse_and_is_shared_between_consumers() {
    unsafe {
        let (context, texture) = fixture();
        let source = crate::live_frames::Source::new();
        let mut first = source.subscribe().unwrap();
        let mut second = source.subscribe().unwrap();
        let cancel = AtomicBool::new(false);
        publish(&source, &context, &texture, &cancel).unwrap();
        let zeros = [0u16; 16];
        context.UpdateSubresource(&texture, 0, None, zeros.as_ptr().cast(), 16, 32);
        let frame = first.try_next().unwrap().unwrap().frame;
        let other = second.try_next().unwrap().unwrap().frame;
        assert!(std::sync::Arc::ptr_eq(&frame, &other));
        assert_ne!(
            frame.resource.texture().cast::<IUnknown>().unwrap(),
            texture.cast::<IUnknown>().unwrap()
        );
        let preview = crate::capture_readback::with_frame(
            &context,
            frame.resource.texture(),
            &cancel,
            |value| value.to_sdr(80.0, 1.0, &cancel),
        )
        .unwrap();
        assert_eq!(&*preview.pixels, &[255; 16]);
        assert_eq!((frame.resource.width, frame.resource.height), (2, 2));
        let weak = std::sync::Arc::downgrade(&frame);
        drop(frame);
        drop(other);
        drop(first);
        drop(second);
        assert!(weak.upgrade().is_none());
    }
}
#[test]
fn absent_demand_cancellation_and_cross_device_are_rejected() {
    unsafe {
        let (context, texture) = fixture();
        let source = crate::live_frames::Source::new();
        let cancel = AtomicBool::new(false);
        assert!(matches!(
            publish(&source, &context, &texture, &cancel),
            Err(Error::Delivery(crate::live_frames::Error::NoConsumers))
        ));
        let _consumer = source.subscribe().unwrap();
        cancel.store(true, Ordering::Release);
        assert!(matches!(
            publish(&source, &context, &texture, &cancel),
            Err(Error::Cancelled)
        ));
        cancel.store(false, Ordering::Release);
        let (other, _) = fixture();
        assert!(matches!(
            snapshot(&other, &texture, &cancel),
            Err(Error::Device)
        ));
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);
        assert_eq!(bytes(&desc).unwrap(), 32);
        desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        assert_eq!(bytes(&desc).unwrap(), 16);
        desc.SampleDesc.Count = 2;
        assert!(bytes(&desc).is_err());
    }
}
