//! Provenance is rebound to pasted documents; shared read-only leases outlive source windows.
use super::*;
use crate::desktop::{canvas::CaptureSource, State};
use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt, path::PathBuf};
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}

#[test]
fn copied_provenance_rebinds_and_read_only_lease_survives_until_last_owner() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let path = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
        .join(format!("otter-clipboard-lease-{:?}.bin", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
    drop(
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap(),
    );
    let fixture = Fixture(path);
    let lease = Arc::new(
        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(&fixture.0)
            .unwrap(),
    );
    let weak = Arc::downgrade(&lease);
    let mut source = CaptureSource::from_session(r#"{"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq","futureClipboard":{"keep":true}}"#).unwrap();
    source.original_lease = Some(lease);
    // Deliberately absent: preparation must not reopen a recorded source path.
    source.original = Some(PathBuf::from("C:/unopened-clipboard-source.png"));
    source.diagnostic = Some("original capture diagnostic".into());
    let metadata = source.metadata();
    let mut content = content();
    content.capture_source = Some(source);
    let content = Arc::new(content);
    let mut cache = Cache::default();
    cache.published(7, Some(content.clone()));
    let first = State::from_prepared(content.prepare(|| false).unwrap()).unwrap();
    let second = State::from_prepared(content.prepare(|| false).unwrap()).unwrap();
    let stamp = first.canvas.borrow().editor.document_stamp();
    for state in [&first, &second] {
        let canvas = state.canvas.borrow();
        let source = canvas.capture_source().unwrap();
        assert_eq!(source.metadata(), metadata);
        assert_eq!(
            source.diagnostic.as_deref(),
            Some("original capture diagnostic")
        );
        assert!(Arc::ptr_eq(
            source.original_lease.as_ref().unwrap(),
            &weak.upgrade().unwrap()
        ));
    }
    assert!(!stamp.same_document(&second.canvas.borrow().editor));
    drop(content);
    cache.published(8, None);
    assert!(weak.upgrade().is_some());
    assert!(OpenOptions::new().write(true).open(&fixture.0).is_err());
    assert!(std::fs::remove_file(&fixture.0).is_err());
    first
        .canvas
        .borrow_mut()
        .replace_image(Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        })
        .unwrap();
    assert!(first.canvas.borrow().capture_source().is_none());
    assert!(second.canvas.borrow().capture_source().is_some());
    assert!(weak.upgrade().is_some());
    drop(second);
    assert!(weak.upgrade().is_none());
    assert!(OpenOptions::new().write(true).open(&fixture.0).is_ok());
}

#[test]
fn metadata_without_a_lease_never_manufactures_original_file_access() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let mut content = content();
    let mut source = CaptureSource::from_session(
        r#"{"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq"}"#,
    )
    .unwrap();
    source.original = Some(PathBuf::from("C:/unopened-clipboard-source.png"));
    content.capture_source = Some(source);
    let state = State::from_prepared(content.prepare(|| false).unwrap()).unwrap();
    let canvas = state.canvas.borrow();
    let source = canvas.capture_source().unwrap();
    assert!(source.original_lease.is_none());
    assert_eq!(source.original_label(), "HDR（原图文件未恢复）");
}
