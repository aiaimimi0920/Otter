use super::*;
use windows::Win32::Foundation::E_POINTER;
#[test]
fn invalid_monitor_and_cancel_reject_before_capture_or_apartment_initialization() {
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        monitor(HMONITOR::default(), &cancel, |_| Ok(())),
        Err(Error::InvalidMonitor)
    ));
    cancel.store(true, Ordering::Release);
    assert!(matches!(
        monitor(HMONITOR::default(), &cancel, |_| Ok(())),
        Err(Error::Cancelled)
    ));
}
#[test]
fn pool_layout_and_successful_null_interface_are_distinct_from_api_failures() {
    let size = SizeInt32 {
        Width: 100,
        Height: 50,
    };
    assert_eq!(pool_bytes(size).unwrap(), 80000);
    assert!(same_size(size, size));
    assert!(!same_size(size, SizeInt32 { Width: 99, ..size }));
    for (width, height) in [(0, 1), (-1, 1), (1, -1), (i32::MAX, i32::MAX)] {
        assert!(pool_bytes(SizeInt32 {
            Width: width,
            Height: height
        })
        .is_err());
    }
    let null: windows::core::Result<Direct3D11CaptureFrame> =
        unsafe { windows::core::Type::from_abi(std::ptr::null_mut()) };
    assert!(matches!(pending(null), Ok(None)));
    let error = pending::<()>(Err(windows::core::Error::from_hresult(E_POINTER)));
    assert_eq!(error.unwrap_err().code(), E_POINTER);
}
#[test]
fn frame_wait_is_bounded_and_cancellation_does_not_fetch_more_frames() {
    let cancel = AtomicBool::new(false);
    let mut calls = 0;
    let result: Result<(), _> = wait(&cancel, Duration::ZERO, || {
        calls += 1;
        Ok(None)
    });
    assert!(matches!(result, Err(Error::Timeout)));
    assert_eq!(calls, 1);
    let result: Result<(), _> = wait(&cancel, Duration::from_secs(1), || {
        calls += 1;
        cancel.store(true, Ordering::Release);
        Ok(None)
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(calls, 2);
    cancel.store(false, Ordering::Release);
    assert_eq!(
        wait(&cancel, Duration::from_secs(1), || Ok(Some(7))).unwrap(),
        7
    );
}

#[test]
fn local_region_is_validated_without_clipping_or_capture() {
    use crate::capture_readback::Region;
    let size = SizeInt32 {
        Width: 100,
        Height: 50,
    };
    let region = Region {
        left: 90,
        top: 40,
        width: 10,
        height: 10,
    };
    assert!(validate_region(size, None).is_ok());
    assert!(validate_region(size, Some(region)).is_ok());
    for region in [
        Region { left: 91, ..region },
        Region { top: 41, ..region },
        Region { width: 0, ..region },
        Region {
            height: 0,
            ..region
        },
        Region {
            left: u32::MAX,
            ..region
        },
    ] {
        assert!(matches!(
            validate_region(size, Some(region)),
            Err(Error::Dimensions)
        ));
    }
    assert!(validate_region(SizeInt32 { Width: -1, ..size }, None).is_err());
}
