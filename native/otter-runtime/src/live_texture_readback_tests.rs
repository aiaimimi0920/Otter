//! WARP 真 GPU 资源读回；合成像素，不捕获用户桌面。
use super::*;
use windows::Win32::{Foundation::HMODULE, Graphics::Direct3D::D3D_DRIVER_TYPE_WARP};
unsafe fn fixture() -> (
    ID3D11DeviceContext,
    ID3D11Texture2D,
    Arc<Frame<Texture>>,
    Vec<u8>,
) {
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
    let context = context.unwrap();
    let protection: ID3D11Multithread = context.cast().unwrap();
    let _ = protection.SetMultithreadProtected(true);
    let desc = D3D11_TEXTURE2D_DESC {
        Width: 3,
        Height: 2,
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
    let bytes = vec![
        0, 0, 255, 255, 0, 128, 0, 128, 0, 0, 0, 0, 255, 0, 0, 255, 1, 2, 3, 255, 4, 5, 6, 255,
    ];
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: bytes.as_ptr().cast(),
        SysMemPitch: 12,
        SysMemSlicePitch: 24,
    };
    let mut texture = None;
    device
        .unwrap()
        .CreateTexture2D(&desc, Some(&initial), Some(&mut texture))
        .unwrap();
    let texture = texture.unwrap();
    let source = crate::live_frames::Source::new();
    let mut consumer = source.subscribe().unwrap();
    super::super::publish(&source, &context, &texture, &AtomicBool::new(false)).unwrap();
    let frame = consumer.try_next().unwrap().unwrap().frame;
    source.close();
    (context, texture, frame, bytes)
}
#[test]
fn exact_bgra_survives_pool_reuse_source_close_and_background_handoff() {
    unsafe {
        let (context, source, frame, bytes) = fixture();
        context.UpdateSubresource(&source, 0, None, [0u8; 24].as_ptr().cast(), 12, 24);
        let weak = Arc::downgrade(&frame);
        let output = std::thread::spawn(move || read(frame, &AtomicBool::new(false)))
            .join()
            .unwrap()
            .unwrap();
        assert_eq!((output.width, output.height), (3, 2));
        assert_eq!(&*output.pixels, bytes);
        assert!(weak.upgrade().is_none());
    }
}
#[test]
fn rejects_cancel_metadata_unprotected_context_and_budget_without_leaking() {
    unsafe {
        let (context, _, mut frame, _) = fixture();
        assert!(matches!(
            read(frame.clone(), &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
        let protection: ID3D11Multithread = context.cast().unwrap();
        let _ = protection.SetMultithreadProtected(false);
        assert!(matches!(
            read(frame.clone(), &AtomicBool::new(false)),
            Err(Error::Unprotected)
        ));
        let _ = protection.SetMultithreadProtected(true);
        let budget = crate::byte_budget::Budget::new(24);
        assert!(matches!(
            read_inner(
                &frame.resource,
                &AtomicBool::new(false),
                |n| budget.reserve(n),
                None
            ),
            Err(Error::Budget)
        ));
        assert_eq!(budget.used(), 0);
        let resource = &mut Arc::get_mut(&mut frame).unwrap().resource;
        resource.width += 1;
        assert!(matches!(
            read(frame.clone(), &AtomicBool::new(false)),
            Err(Error::Layout)
        ));
    }
}
#[test]
fn mapping_budget_failure_releases_leases_and_later_readback_still_succeeds() {
    unsafe {
        let (_, _, frame, bytes) = fixture();
        // 注入映射后无法扩展的 staging 租约，确保覆盖 Map 后错误路径，不依赖驱动 padding。
        let budget = crate::byte_budget::Budget::new(0);
        let calls = std::cell::Cell::new(0);
        let result = read_inner(
            &frame.resource,
            &AtomicBool::new(false),
            |n| {
                let count = calls.replace(calls.get() + 1);
                if count == 0 {
                    budget.reserve(0)
                } else {
                    crate::pixels::reserve(n)
                }
            },
            None,
        );
        assert!(matches!(result, Err(Error::Budget)));
        assert_eq!(budget.used(), 0);
        assert_eq!(
            &*read(frame, &AtomicBool::new(false)).unwrap().pixels,
            bytes
        );
    }
}
#[test]
fn nonblocking_wait_distinguishes_timeout_cancel_and_device_failure() {
    let cancel = AtomicBool::new(false);
    let busy = || {
        Err::<(), _>(windows::core::Error::from_hresult(
            DXGI_ERROR_WAS_STILL_DRAWING,
        ))
    };
    assert!(matches!(
        wait(&cancel, Duration::ZERO, busy),
        Err(Error::Timeout)
    ));
    assert!(matches!(
        wait(&cancel, Duration::from_secs(1), || {
            cancel.store(true, Ordering::Release);
            busy()
        }),
        Err(Error::Cancelled)
    ));
    cancel.store(false, Ordering::Release);
    assert!(matches!(
        wait(&cancel, Duration::from_secs(1), || Err::<(), _>(
            windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)
        )),
        Err(Error::Api(_))
    ));
}
#[test]
fn scratch_reuses_staging_for_matching_layout_and_clears_explicitly() {
    let (_, _, frame, bytes) = unsafe { fixture() };
    let cancel = AtomicBool::new(false);
    let budget = crate::byte_budget::Budget::new(4096);
    let mut scratch = Scratch::default();
    let first = unsafe {
        read_inner(
            &frame.resource,
            &cancel,
            |n| budget.reserve(n),
            Some(&mut scratch.0),
        )
    }
    .unwrap();
    let ptr = scratch.raw_staging().unwrap();
    let retained = scratch.0.as_ref().unwrap().lease.bytes();
    assert!(retained >= bytes.len());
    assert_eq!(budget.used(), retained + first.pixels.len());
    assert_eq!(&*first.pixels, bytes);
    drop(first);
    assert_eq!(budget.used(), retained);
    // 仅留输出所需余量；同规格复用不得再次为 staging 申请预算。
    let occupied = budget.reserve(4096 - retained - bytes.len()).unwrap();
    let second = unsafe {
        read_inner(
            &frame.resource,
            &cancel,
            |n| budget.reserve(n),
            Some(&mut scratch.0),
        )
    }
    .unwrap();
    assert_eq!(scratch.raw_staging(), Some(ptr));
    assert_eq!(&*second.pixels, bytes);
    assert_eq!(budget.used(), 4096);
    drop(occupied);
    drop(second);
    assert_eq!(budget.used(), retained);
    scratch.clear();
    assert!(scratch.raw_staging().is_none());
    assert_eq!(budget.used(), 0);
}

#[test]
fn scratch_replacement_releases_old_device_lease_and_drop_releases_new_lease() {
    let (_, _, frame, _) = unsafe { fixture() };
    let (_, _, replacement, bytes) = unsafe { fixture() };
    let budget = crate::byte_budget::Budget::new(4096);
    let mut scratch = Scratch::default();
    let cancel = AtomicBool::new(false);
    let read = |frame: &Texture, scratch: &mut Scratch| unsafe {
        read_inner(frame, &cancel, |n| budget.reserve(n), Some(&mut scratch.0))
    };
    drop(read(&frame.resource, &mut scratch).unwrap());
    let retained = budget.used();
    let occupied = budget.reserve(4096 - retained - bytes.len()).unwrap();
    let output = read(&replacement.resource, &mut scratch).unwrap();
    assert_eq!(&*output.pixels, bytes);
    assert_eq!(
        budget.used(),
        4096,
        "old staging was released before replacement"
    );
    drop(output);
    drop(occupied);
    assert_eq!(budget.used(), retained);
    drop(scratch);
    assert_eq!(budget.used(), 0);
}

#[test]
fn scratch_failed_mapping_growth_drops_staging_before_releasing_budget() {
    let (_, _, frame, bytes) = unsafe { fixture() };
    let budget = crate::byte_budget::Budget::new(0);
    let mut scratch = Scratch::default();
    let calls = std::cell::Cell::new(0);
    let result = unsafe {
        read_inner(
            &frame.resource,
            &AtomicBool::new(false),
            |n| {
                if calls.replace(calls.get() + 1) == 0 {
                    budget.reserve(0)
                } else {
                    crate::pixels::reserve(n)
                }
            },
            Some(&mut scratch.0),
        )
    };
    assert!(matches!(result, Err(Error::Budget)));
    assert!(scratch.raw_staging().is_none());
    assert_eq!(budget.used(), 0);
    assert_eq!(
        &*scratch.read(frame, &AtomicBool::new(false)).unwrap().pixels,
        bytes
    );
}
