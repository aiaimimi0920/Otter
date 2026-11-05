use super::*;
use sha2::{Digest, Sha256};
fn hdr_frame() -> crate::capture_color::HdrFrame {
    let bytes: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    crate::capture_color::ScRgb::new(&bytes, 1, 1, 8)
        .unwrap()
        .to_hdr(0.01, 1000.0, 1.0, &AtomicBool::new(false))
        .unwrap()
}
#[test]
fn active_original_lease_blocks_replacement_and_cache_retirement_until_released() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let (path, lease) = stage_hdr(
        &directory.0,
        "leased",
        &hdr_frame(),
        &cancel,
        Default::default(),
    )
    .unwrap()
    .commit_leased(&cancel)
    .unwrap();
    assert!(OpenOptions::new().write(true).open(&path).is_err());
    assert!(fs::remove_file(&path).is_err());
    let future = std::time::SystemTime::now() + std::time::Duration::from_secs(8 * 86400);
    let report = cleanup::run(&directory.cache(), future, &cancel).unwrap();
    assert_eq!(report.removed, 0);
    assert!(path.is_file());
    // 只租赁原图，不占缓存排他锁；其它截图仍可暂存。
    drop(
        stage_hdr(
            &directory.0,
            "next",
            &hdr_frame(),
            &cancel,
            Default::default(),
        )
        .unwrap(),
    );
    assert_eq!(directory.entries(), 3);
    drop(lease);
    assert_eq!(
        cleanup::run(&directory.cache(), future, &cancel)
            .unwrap()
            .removed,
        1
    );
    assert!(!path.exists());
    assert_eq!(directory.entries(), 1);
}
#[test]
fn streamed_hdr_cache_publishes_exact_hash_identity_and_full_rgb16() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let frame = hdr_frame();
    let staged = stage_hdr(
        &directory.0,
        "HDR 原图",
        &frame,
        &cancel,
        Default::default(),
    )
    .unwrap();
    assert_eq!(directory.entries(), 2);
    let path = staged.commit(&cancel).unwrap();
    assert_eq!(directory.entries(), 3);
    let bytes = fs::read(&path).unwrap();
    let entry = fs::read_dir(directory.cache())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .unwrap();
    let receipt: receipt::Receipt = serde_json::from_slice(&fs::read(entry).unwrap()).unwrap();
    assert_eq!(receipt.bytes, bytes.len() as u64);
    assert_eq!(receipt.sha256, <[u8; 32]>::from(Sha256::digest(&bytes)));
    assert_eq!(receipt.name, path.file_name().unwrap().to_str().unwrap());
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.bit_depth, png::BitDepth::Sixteen);
    assert_eq!(&pixels[..info.buffer_size()], &*frame.rgb16_be);
}
#[test]
fn hdr_cache_quota_cancel_and_encoder_error_do_not_leave_unowned_pngs() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let mut frame = hdr_frame();
    let policy = crate::cache_policy::Policy {
        temp_cache_max_bytes: 32 * 1024 * 1024,
        ..Default::default()
    };
    let oversized = crate::capture_color::HdrFrame {
        width: 2048,
        height: 3000,
        rgb16_be: vec![0; 2048 * 3000 * 6].into(),
        ..hdr_frame()
    };
    assert!(stage_hdr(&directory.0, "limit", &oversized, &cancel, policy).is_err());
    drop(oversized);
    assert!(!directory.cache().exists());
    let staged = stage_hdr(&directory.0, "cancel", &frame, &cancel, Default::default()).unwrap();
    cancel.store(true, Ordering::Release);
    assert!(staged.commit(&cancel).is_err());
    assert_eq!(directory.entries(), 1);
    cancel.store(false, Ordering::Release);
    frame.analysis.maximum_nits = f32::NAN;
    assert!(stage_hdr(&directory.0, "invalid", &frame, &cancel, Default::default()).is_err());
    assert_eq!(directory.entries(), 1);
    frame.analysis.maximum_nits = 160.0;
    drop(
        stage_hdr(
            &directory.0,
            "abandoned",
            &frame,
            &cancel,
            Default::default(),
        )
        .unwrap(),
    );
    assert_eq!(directory.entries(), 1);
}
