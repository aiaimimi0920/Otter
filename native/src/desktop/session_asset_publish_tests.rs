use super::*;
fn fixture() -> (std::path::PathBuf, Root, Pixels) {
    let directory =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-archive-{}",
            crate::desktop::document_state::new_unit_id().unwrap()
        ));
    std::fs::create_dir(&directory).unwrap();
    let root = Root::open(&directory).unwrap();
    let image = crate::desktop::image::Image {
        width: 2,
        height: 1,
        pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
    };
    let png = crate::desktop::export::png_raw(&image, &AtomicBool::new(false)).unwrap();
    (directory, root, png)
}
#[test]
fn resource_publication_is_reusable_and_holds_file_until_session_commit() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, png) = fixture();
    let cancel = AtomicBool::new(false);
    let first = root
        .publish_image("Test /贴图", Slot::Source, &png, &cancel)
        .unwrap();
    let inline = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let second = root
        .publish_inline("Test /贴图", Slot::Source, inline.as_str(), &cancel)
        .unwrap();
    assert_eq!(first.reference(), second.reference());
    assert_eq!(std::fs::read(first.reference()).unwrap(), png.as_ref());
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .open(first.reference())
        .is_err());
    let preview = root
        .publish_image("Test /贴图", Slot::Preview, &png, &cancel)
        .unwrap();
    assert_ne!(preview.reference(), first.reference());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
    drop((first, second, preview, root));
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn corrupt_existing_resource_is_not_overwritten_and_cancel_leaves_no_file() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, png) = fixture();
    let cancel = AtomicBool::new(false);
    let name = filename("unit", &Slot::Source, &png, &cancel).unwrap();
    let corrupt = vec![0u8; png.len()];
    std::fs::write(directory.join(&name), &corrupt).unwrap();
    assert!(root
        .publish_image("unit", Slot::Source, &png, &cancel)
        .is_err());
    assert_eq!(std::fs::read(directory.join(&name)).unwrap(), corrupt);
    assert!(root
        .publish_image("other", Slot::Source, &png, &AtomicBool::new(true))
        .is_err());
    assert!(root
        .publish_image("bad", Slot::Source, &vec![1, 2, 3].into(), &cancel)
        .is_err());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    drop(root);
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn naming_matches_legacy_fingerprint_and_sanitization() {
    let cancel = AtomicBool::new(false);
    assert_eq!(
        filename("__Ab C__", &Slot::Source, b"hello", &cancel).unwrap(),
        "ab_c_source_a430d84680aabd0b.png"
    );
    assert_eq!(
        filename("中文", &Slot::Preview, b"", &cancel).unwrap(),
        "hook_preview_cbf29ce484222325.png"
    );
}

#[test]
fn concurrent_publication_reuses_one_exact_resource_without_temporary_leaks() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, png) = fixture();
    let root = std::sync::Arc::new(root);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let barrier = barrier.clone();
            let png = png.clone();
            std::thread::spawn(move || {
                barrier.wait();
                root.publish_image("shared", Slot::Source, &png, &AtomicBool::new(false))
                    .unwrap()
            })
        })
        .collect();
    barrier.wait();
    let published: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(published[0].reference(), published[1].reference());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    drop((published, root));
    std::fs::remove_dir_all(directory).unwrap();
}

// 模拟改名已可见但发布者 DELETE 句柄尚未关闭的窗口，不放宽 Published 的共享模式。
fn delete_access(directory: &std::path::Path, png: &Pixels) -> File {
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Storage::FileSystem::{
        DELETE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let name = filename("shared", &Slot::Source, png, &AtomicBool::new(false)).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, png.as_ref()).unwrap();
    std::fs::OpenOptions::new()
        .access_mode(DELETE.0)
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
        .open(path)
        .unwrap()
}

#[test]
fn publication_waits_for_existing_publishers_delete_handle_without_weakening_pin() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, png) = fixture();
    let locked = delete_access(&directory, &png);
    let reference =
        directory.join(filename("shared", &Slot::Source, &png, &AtomicBool::new(false)).unwrap());
    assert!(root.open_image(reference.to_str().unwrap()).is_err());
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        drop(locked);
    });
    let result = root.publish_image("shared", Slot::Source, &png, &AtomicBool::new(false));
    release.join().unwrap();
    let success = result.is_ok();
    if let Ok(ref published) = result {
        assert_eq!(std::fs::read(published.reference()).unwrap(), png.as_ref());
        assert!(std::fs::OpenOptions::new()
            .write(true)
            .open(published.reference())
            .is_err());
        assert!(std::fs::remove_file(published.reference()).is_err());
    }
    drop((result, root));
    std::fs::remove_dir_all(directory).unwrap();
    assert!(
        success,
        "temporary DELETE access must not reject identical publication"
    );
}

#[test]
fn publication_sharing_wait_is_bounded_and_cancellable() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (directory, root, png) = fixture();
    let locked = delete_access(&directory, &png);
    let cancel = AtomicBool::new(false);
    let started = std::time::Instant::now();
    let result = root.publish_image("shared", Slot::Source, &png, &cancel);
    let error = result
        .err()
        .expect("held DELETE access must not yield a pin");
    assert_eq!(
        error.code(),
        windows::core::HRESULT::from_win32(windows::Win32::Foundation::ERROR_SHARING_VIOLATION.0)
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    assert!(error.message().contains("reopen failed"));
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(30));
            cancel.store(true, Ordering::Release);
        });
        root.publish_image("shared", Slot::Source, &png, &cancel)
    });
    assert_eq!(result.err().unwrap().code(), E_ABORT);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    drop((locked, root));
    std::fs::remove_dir_all(directory).unwrap();
}
