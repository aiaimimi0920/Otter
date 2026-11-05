use super::*;
use crate::byte_budget::Budget;
fn size(width: i32, height: i32) -> SizeInt32 {
    SizeInt32 {
        Width: width,
        Height: height,
    }
}
#[test]
fn only_window_targets_resize_and_zero_content_suspends_without_allocation() {
    assert_eq!(plan(false, size(2, 2), size(2, 2)).unwrap(), Plan::Same);
    assert!(matches!(
        plan(false, size(2, 2), size(3, 3)),
        Err(Error::Resized)
    ));
    assert_eq!(plan(true, size(2, 2), size(3, 3)).unwrap(), Plan::Recreate);
    for next in [size(0, 3), size(3, 0), size(0, 0)] {
        assert_eq!(plan(true, size(2, 2), next).unwrap(), Plan::Suspend);
    }
    for next in [size(-1, 2), size(2, -1), size(i32::MAX, i32::MAX)] {
        assert!(matches!(
            plan(true, size(2, 2), next),
            Err(Error::Dimensions)
        ));
    }
}
#[test]
fn peak_budget_and_recreate_failure_do_not_commit_size_or_leak_candidate() {
    let budget = Budget::new(100);
    let mut lease = budget.reserve(32);
    let mut current = size(2, 2);
    assert!(matches!(
        recreate(
            &mut current,
            &mut lease,
            size(3, 3),
            |bytes| budget.reserve(bytes),
            || panic!("over-budget rebuild")
        ),
        Err(Error::Budget)
    ));
    assert_eq!(budget.used(), 32);
    assert!(matches!(
        recreate(
            &mut current,
            &mut lease,
            size(2, 3),
            |bytes| budget.reserve(bytes),
            || Err(Error::Cancelled)
        ),
        Err(Error::Cancelled)
    ));
    assert_eq!(current, size(2, 2));
    assert_eq!(budget.used(), 32);
    recreate(
        &mut current,
        &mut lease,
        size(2, 3),
        |bytes| budget.reserve(bytes),
        || {
            assert_eq!(budget.used(), 80);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(current, size(2, 3));
    assert_eq!(budget.used(), 48);
    drop(lease.take());
    assert_eq!(budget.used(), 0);
    assert!(matches!(
        recreate(
            &mut current,
            &mut lease,
            size(2, 2),
            |_| panic!("closed owner allocation"),
            || panic!("closed owner rebuild")
        ),
        Err(Error::Cancelled)
    ));
}

#[test]
fn real_warp_frame_pool_recreates_without_starting_any_capture_session() {
    use windows::{
        core::Interface,
        Graphics::{
            Capture::Direct3D11CaptureFramePool,
            DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        },
        Win32::{
            Foundation::HMODULE,
            Graphics::{Direct3D::D3D_DRIVER_TYPE_WARP, Direct3D11::*, Dxgi::IDXGIDevice},
            System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
        },
    };
    let _apartment = super::super::super::lifetime::Apartment::new().unwrap();
    unsafe {
        let mut device = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_WARP,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
        .unwrap();
        let dxgi: IDXGIDevice = device.unwrap().cast().unwrap();
        let winrt: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)
            .unwrap()
            .cast()
            .unwrap();
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size(8, 8),
        )
        .unwrap();
        let resources = super::super::super::lifetime::Resources {
            pool: Some(pool.clone()),
            session: None,
        };
        let budget = Budget::new(32 * 32 * 8 * 2);
        let mut current = size(8, 8);
        let mut lease = budget.reserve(pool_bytes(current).unwrap());
        for next in [size(16, 24), size(32, 8), size(4, 4)] {
            recreate(
                &mut current,
                &mut lease,
                next,
                |bytes| budget.reserve(bytes),
                || {
                    pool.Recreate(&winrt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, next)
                        .map_err(Error::Api)
                },
            )
            .unwrap();
            assert_eq!(current, next);
            assert_eq!(budget.used(), pool_bytes(next).unwrap());
        }
        drop(resources);
        drop(lease);
        assert_eq!(budget.used(), 0);
    }
}
