//! 使用独立预算，证明编码前准入及错误/成功后的租约回收，不耗尽全局像素池。
use super::*;
#[test]
fn projection_encoding_budget_precedes_wic_and_releases_after_success_or_failure() {
    let _com = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let bitmap = unsafe {
        wic.CreateBitmapFromMemory(2, 1, &GUID_WICPixelFormat32bppPBGRA, 8, &[255; 8])
            .unwrap()
    };
    let output = Output {
        size: (2, 1),
        projection: true,
    };
    let expected = 9 + 64 + 64 * 1024;
    let cancel = AtomicBool::new(false);
    let output_budget = crate::byte_budget::Budget::new(8);
    let composed = || ComposedImage {
        bitmap: bitmap.clone(),
        _lease: output_budget.reserve(8).unwrap(),
    };
    for limit in [expected - 1, expected] {
        let budget = crate::byte_budget::Budget::new(limit);
        let requested = std::cell::Cell::new(0);
        let result = output.bitmap_with_budget(
            &wic,
            composed(),
            &cancel,
            |bytes| {
                requested.set(bytes);
                budget.reserve(bytes)
            },
            |stream, bytes| {
                assert_eq!(
                    output_budget.used(),
                    0,
                    "composed output released before copy"
                );
                assert_eq!(budget.used(), expected, "encoded HGlobal remains budgeted");
                stream_pixels(stream, bytes)
            },
        );
        assert_eq!(requested.get(), expected);
        if limit < expected {
            assert_eq!(
                result.unwrap_err().code(),
                windows::Win32::Foundation::E_OUTOFMEMORY
            );
        } else {
            assert_eq!(&result.unwrap()[..8], b"\x89PNG\r\n\x1a\n");
        }
        assert_eq!(budget.used(), 0);
        assert_eq!(output_budget.used(), 0);
    }
    cancel.store(true, Ordering::Release);
    assert_eq!(
        output
            .bitmap_with_budget(
                &wic,
                composed(),
                &cancel,
                |_| panic!("cancelled before reserve"),
                |_, _| panic!("cancelled before copy"),
            )
            .unwrap_err()
            .code(),
        windows::Win32::Foundation::E_ABORT
    );
    let changed = Output {
        size: (1, 1),
        projection: true,
    };
    assert_eq!(
        changed
            .bitmap_with_budget(
                &wic,
                composed(),
                &AtomicBool::new(false),
                |_| panic!("size mismatch before reserve"),
                |_, _| panic!("size mismatch before copy"),
            )
            .unwrap_err()
            .code(),
        E_INVALIDARG
    );
    assert_eq!(output_budget.used(), 0);
}

#[test]
fn ordinary_source_encoding_releases_composed_budget_before_png_copy() {
    let _com = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let budget = crate::byte_budget::Budget::new(8);
    let bitmap = ComposedImage {
        bitmap: unsafe {
            wic.CreateBitmapFromMemory(2, 1, &GUID_WICPixelFormat32bppPBGRA, 8, &[255; 8])
                .unwrap()
        },
        _lease: budget.reserve(8).unwrap(),
    };
    let output = Output {
        size: (2, 1),
        projection: false,
    };
    let png = output
        .bitmap_with_budget(
            &wic,
            bitmap,
            &AtomicBool::new(false),
            |_| panic!("ordinary workflow does not reserve projection encoding budget"),
            |stream, bytes| {
                assert_eq!(budget.used(), 0, "ordinary output released before copy");
                stream_pixels(stream, bytes)
            },
        )
        .unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(budget.used(), 0);
}

#[test]
fn projection_encoded_stream_copy_failure_releases_all_output_leases() {
    encoded_copy_failure(false);
}

#[test]
fn projection_cancel_after_stream_copy_discards_late_pixels_and_output_leases() {
    encoded_copy_failure(true);
}

fn encoded_copy_failure(cancel_after_copy: bool) {
    let _com = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let output = Output {
        size: (2, 1),
        projection: true,
    };
    let output_budget = crate::byte_budget::Budget::new(8);
    let encoded_bound = 9 + 64 + 64 * 1024;
    let encoded_budget = crate::byte_budget::Budget::new(encoded_bound);
    let bitmap = ComposedImage {
        bitmap: unsafe {
            wic.CreateBitmapFromMemory(2, 1, &GUID_WICPixelFormat32bppPBGRA, 8, &[255; 8])
                .unwrap()
        },
        _lease: output_budget.reserve(8).unwrap(),
    };
    let cancel = AtomicBool::new(false);
    let copied = std::cell::RefCell::new(None);
    let called = std::cell::Cell::new(false);
    let error = output
        .bitmap_with_budget(
            &wic,
            bitmap,
            &cancel,
            |bytes| {
                assert_eq!(bytes, encoded_bound);
                encoded_budget.reserve(bytes)
            },
            |stream, bytes| {
                called.set(true);
                assert!(bytes > 8, "WIC must have produced an encoded stream");
                assert_eq!(output_budget.used(), 0);
                assert_eq!(encoded_budget.used(), encoded_bound);
                if !cancel_after_copy {
                    return Err(Error::new(
                        windows::Win32::Foundation::E_FAIL,
                        "Injected encoded stream copy failure",
                    ));
                }
                let pixels = stream_pixels(stream, bytes)?;
                assert_eq!(&pixels[..8], b"\x89PNG\r\n\x1a\n");
                *copied.borrow_mut() = Some(pixels.downgrade());
                cancel.store(true, Ordering::Release);
                Ok(pixels)
            },
        )
        .unwrap_err();
    assert!(
        called.get(),
        "failure is after encoding, not early admission"
    );
    assert_eq!(
        error.code(),
        if cancel_after_copy {
            windows::Win32::Foundation::E_ABORT
        } else {
            windows::Win32::Foundation::E_FAIL
        }
    );
    assert_eq!(output_budget.used(), 0);
    assert_eq!(encoded_budget.used(), 0);
    if cancel_after_copy {
        assert!(copied.into_inner().unwrap().upgrade().is_none());
    }
}
