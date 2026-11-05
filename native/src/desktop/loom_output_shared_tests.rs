//! 只创建本测试的命名映射，不打开真实 Loom 资源。
use super::*;
use serde_json::json;
use std::sync::atomic::AtomicUsize;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    handle: HANDLE,
    name: String,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let name = format!(
            "Loom_Buffer_native_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                bytes.len() as u32,
                PCWSTR(wide.as_ptr()),
            )
            .unwrap();
            let view = MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, bytes.len());
            assert!(!view.Value.is_null());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), view.Value.cast(), bytes.len());
            UnmapViewOfFile(view).unwrap();
            Self { handle, name }
        }
    }
    fn descriptor(&self, width: u32, height: u32) -> Value {
        json!({"kind":"shared_memory","handle":self.name,"format":"rgba8","width":width,"height":height,"size":width as u64*height as u64*4})
    }
}
#[test]
fn shared_rgba_is_copied_premultiplied_and_local_handles_are_released() {
    let fixture = Fixture::new(&[200, 100, 50, 128, 30, 20, 10, 255]);
    let descriptor = Descriptor::parse(&fixture.descriptor(2, 1)).unwrap();
    let image = read(&descriptor, &AtomicBool::new(false)).unwrap();
    assert_eq!(&*image.pixels, &[25, 50, 100, 128, 10, 20, 30, 255]);
    assert_eq!((image.width, image.height), (2, 1));
    drop(fixture);
    // 若 read 泄漏本地映射句柄，该名字仍能再次打开。
    assert!(read(&descriptor, &AtomicBool::new(false)).is_err());
    assert_eq!(image.pixels[3], 128);
}
#[test]
fn malformed_descriptor_and_oversized_view_are_rejected() {
    let fixture = Fixture::new(&[0; 4]);
    for (field, bad) in [
        ("handle", json!("foreign")),
        ("format", json!("bgra8")),
        ("width", json!(0)),
        ("size", json!(3)),
        ("height", json!(4294967296u64)),
    ] {
        let mut value = fixture.descriptor(1, 1);
        value[field] = bad;
        assert!(Descriptor::parse(&value).is_err());
    }
    let too_large = Descriptor::parse(&fixture.descriptor(4096, 1)).unwrap();
    assert!(read(&too_large, &AtomicBool::new(false)).is_err());
}
#[test]
fn cancelled_read_does_not_open_mapping() {
    let fixture = Fixture::new(&[0; 4]);
    let descriptor = Descriptor::parse(&fixture.descriptor(1, 1)).unwrap();
    drop(fixture);
    let error = match read(&descriptor, &AtomicBool::new(true)) {
        Ok(_) => panic!("cancel"),
        Err(error) => error,
    };
    assert_eq!(error.code(), E_ABORT);
}

#[test]
fn delivery_keeps_copied_pixels_after_provider_releases_mapping() {
    let fixture = Fixture::new(&[200, 100, 50, 128]);
    let formal = fixture.descriptor(1, 1);
    let descriptor = Descriptor::parse(&formal).unwrap();
    let unit = crate::workspace_id::UnitId::parse("unit").unwrap();
    let request = crate::graph_requests::Request {
        id: crate::workspace_id::UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let mut delivery = super::super::loom_delivery::Delivery::prepare(
        unit,
        request,
        &json!({"output_image":formal}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(delivery.result().is_ok());
    assert_eq!(delivery.pending_release_count(), 1);
    delivery
        .release_with(
            crate::workspace_id::UnitId::parse("release:fixture").unwrap(),
            |_, _| {
                drop(fixture);
                Ok(crate::loom_art_control::Outcome::Succeeded)
            },
        )
        .unwrap();
    assert_eq!(delivery.pending_release_count(), 0);
    assert!(read(&descriptor, &AtomicBool::new(false)).is_err());
    let super::super::loom_output_batch::Output::Image(image) =
        &delivery.result().unwrap().primary().value
    else {
        panic!("image");
    };
    assert_eq!(&*image.pixels, &[25, 50, 100, 128]);
    let pointer = image.pixels.as_ptr();
    let (ports, primary) = delivery.take_released().unwrap().into_ports();
    let super::super::loom_output_batch::Output::Image(image) = &ports[primary].value else {
        panic!("image");
    };
    assert_eq!(image.pixels.as_ptr(), pointer);
    assert_eq!(&*image.pixels, &[25, 50, 100, 128]);
}
