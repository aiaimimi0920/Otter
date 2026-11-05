use super::*;
use windows::Win32::{
    Foundation::{E_FAIL, HMODULE},
    Graphics::Direct3D::D3D_DRIVER_TYPE_WARP,
};
fn description() -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
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
    }
}
unsafe fn fixture() -> (ID3D11DeviceContext, ID3D11Texture2D) {
    let mut device = None;
    let mut context = None;
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
    let data = [0x3c00u16; 16];
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: data.as_ptr().cast(),
        SysMemPitch: 16,
        SysMemSlicePitch: 32,
    };
    let mut texture = None;
    device
        .unwrap()
        .CreateTexture2D(&description(), Some(&initial), Some(&mut texture))
        .unwrap();
    (context.unwrap(), texture.unwrap())
}
#[test]
fn warp_texture_reads_into_sdr_and_hdr_without_desktop_capture() {
    unsafe {
        let (context, texture) = fixture();
        let cancel = AtomicBool::new(false);
        let output = with_frame(&context, &texture, &cancel, |frame| {
            frame.to_sdr(80.0, 1.0, &cancel)
        })
        .unwrap();
        assert_eq!((output.width, output.height), (2, 2));
        assert_eq!(&*output.pixels, &[255; 16]);
        let hdr = with_frame(&context, &texture, &cancel, |frame| {
            frame.to_hdr(0.0, 1000.0, 1.0, &cancel)
        })
        .unwrap();
        assert_eq!(hdr.rgb16_be.len(), 24);
        assert!((hdr.analysis.maximum_nits - 80.0).abs() < 0.01);
        let (other, _) = fixture();
        assert!(matches!(
            with_frame(&other, &texture, &cancel, |_| Ok(())),
            Err(Error::Device)
        ));
    }
}
#[test]
fn validation_timeout_and_cancel_do_not_retry_permanent_errors() {
    assert_eq!(validate(&description()).unwrap(), 32);
    for desc in [
        D3D11_TEXTURE2D_DESC {
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            ..description()
        },
        D3D11_TEXTURE2D_DESC {
            Width: 0,
            ..description()
        },
        D3D11_TEXTURE2D_DESC {
            ArraySize: 2,
            ..description()
        },
        D3D11_TEXTURE2D_DESC {
            MipLevels: 2,
            ..description()
        },
        D3D11_TEXTURE2D_DESC {
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 2,
                Quality: 0,
            },
            ..description()
        },
    ] {
        assert!(matches!(validate(&desc), Err(Error::Layout)));
    }
    let cancel = AtomicBool::new(false);
    let mut calls = 0;
    let result: Result<(), _> = wait(&cancel, Duration::ZERO, || {
        calls += 1;
        Err(windows::core::Error::from_hresult(
            DXGI_ERROR_WAS_STILL_DRAWING,
        ))
    });
    assert!(matches!(result, Err(Error::Timeout)));
    assert_eq!(calls, 1);
    let result: Result<(), _> = wait(&cancel, Duration::from_secs(1), || {
        calls += 1;
        Err(windows::core::Error::from_hresult(E_FAIL))
    });
    assert!(matches!(result, Err(Error::Api(_))));
    assert_eq!(calls, 2);
    cancel.store(true, Ordering::Release);
    assert!(matches!(
        wait(&cancel, Duration::from_secs(1), || Ok(())),
        Err(Error::Cancelled)
    ));
}
#[test]
fn mapped_conversion_failure_cancel_and_panic_release_readback_budget() {
    unsafe {
        let (context, texture) = fixture();
        let cancel = AtomicBool::new(false);
        let budget = crate::byte_budget::Budget::new(1024 * 1024);
        assert!(matches!(
            read(&context, &texture, &cancel, |_| Ok(()), |_| None),
            Err(Error::Budget)
        ));
        let result: Result<(), _> = read(
            &context,
            &texture,
            &cancel,
            |_| Err(capture_color::Error::Layout),
            |n| budget.reserve(n),
        );
        assert!(matches!(result, Err(Error::Conversion(_))));
        assert_eq!(budget.used(), 0);
        let result = read(
            &context,
            &texture,
            &cancel,
            |_| {
                cancel.store(true, Ordering::Release);
                Ok(())
            },
            |n| budget.reserve(n),
        );
        assert!(matches!(result, Err(Error::Cancelled)));
        assert_eq!(budget.used(), 0);
        cancel.store(false, Ordering::Release);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<(), _> = read(
                &context,
                &texture,
                &cancel,
                |_| panic!("injected conversion panic"),
                |n| budget.reserve(n),
            );
        }));
        assert!(panic.is_err());
        assert_eq!(budget.used(), 0);
        read(
            &context,
            &texture,
            &cancel,
            |_| Ok(()),
            |n| budget.reserve(n),
        )
        .unwrap();
        assert_eq!(budget.used(), 0);
    }
}

#[test]
fn gpu_region_copies_only_requested_pixels_and_admits_cropped_staging_size() {
    unsafe {
        let (context, texture) = fixture();
        let data = [
            0x3c00u16, 0, 0, 0x3c00, 0, 0x3c00, 0, 0x3c00, 0, 0, 0x3c00, 0x3c00, 0x3c00, 0x3c00,
            0x3c00, 0x3c00,
        ];
        context.UpdateSubresource(&texture, 0, None, data.as_ptr().cast(), 16, 32);
        let cancel = AtomicBool::new(false);
        let region = Region {
            left: 1,
            top: 0,
            width: 1,
            height: 2,
        };
        let budget = crate::byte_budget::Budget::new(1024 * 1024);
        let requested = std::cell::Cell::new(0);
        let output = read_region(
            &context,
            &texture,
            Some(region),
            &cancel,
            |frame| frame.to_sdr(80.0, 1.0, &cancel),
            |bytes| {
                requested.set(bytes);
                budget.reserve(bytes)
            },
        )
        .unwrap();
        assert_eq!(requested.get(), 16);
        assert_eq!(budget.used(), 0);
        assert_eq!((output.width, output.height), (1, 2));
        assert_eq!(&*output.pixels, &[0, 255, 0, 255, 255, 255, 255, 255]);
        let corner = with_region(
            &context,
            &texture,
            Region {
                left: 0,
                top: 1,
                width: 1,
                height: 1,
            },
            &cancel,
            |frame| frame.to_sdr(80.0, 1.0, &cancel),
        )
        .unwrap();
        assert_eq!(&*corner.pixels, &[255, 0, 0, 255]);
        for region in [
            Region { width: 0, ..region },
            Region { left: 2, ..region },
            Region {
                left: u32::MAX,
                ..region
            },
            Region { top: 1, ..region },
        ] {
            let result = read_region(
                &context,
                &texture,
                Some(region),
                &cancel,
                |_| Ok(()),
                |_| panic!("invalid region must not allocate staging"),
            );
            assert!(matches!(result, Err(Error::Layout)));
        }
    }
}
