//! 发送日志异常必须保留原字节；复用真实 session 事务，不伪造持久成功。
use super::*;
#[test]
fn durable_cancellation_is_idempotent_and_cannot_be_replaced_or_revived_after_restart() {
    let (directory, store) = setup();
    let first = prepare(&store, &directory, &selection(1)).unwrap();
    let envelope = first.envelope().clone();
    store.cancel(&first).unwrap();
    let bytes = std::fs::read(directory.path()).unwrap();
    store.cancel(&first).unwrap();
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    assert_eq!(
        prepare(&store, &directory, &selection(1)).err(),
        Some(Error::Cancelled)
    );
    let pending = store.pending(&AtomicBool::new(false)).unwrap();
    assert!(pending[0].cancelled());
    assert_eq!(pending[0].envelope(), &envelope);
    drop((first, pending, store));
    let reopened = directory.store();
    assert!(reopened.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
    assert_eq!(
        prepare(&reopened, &directory, &selection(1)).err(),
        Some(Error::Cancelled)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
}
#[test]
fn external_change_deletion_and_new_recovery_prevent_mutations_without_overwriting() {
    for mutation in [Some(b"external writer".as_slice()), None] {
        let (directory, store) = setup();
        let intent = prepare(&store, &directory, &selection(1)).unwrap();
        if let Some(bytes) = mutation {
            std::fs::write(directory.path(), bytes).unwrap();
        } else {
            std::fs::remove_file(directory.path()).unwrap();
        }
        assert_eq!(
            prepare(&store, &directory, &selection(1)).err(),
            Some(Error::Changed)
        );
        assert_eq!(
            prepare(&store, &directory, &selection(2)).err(),
            Some(Error::Changed)
        );
        assert_eq!(store.cancel(&intent), Err(Error::Changed));
        assert_eq!(
            store.pending(&AtomicBool::new(false)).err(),
            Some(Error::Changed)
        );
        assert_eq!(std::fs::read(directory.path()).ok().as_deref(), mutation);
    }
    let (directory, store) = setup();
    std::fs::write(recovery(&directory, 1), br#"{"version":1,"entries":[]}"#).unwrap();
    assert_eq!(
        prepare(&store, &directory, &selection(1)).err(),
        Some(Error::Changed)
    );
    assert!(!directory.path().exists());
}
fn recovery(directory: &Directory, index: u32) -> PathBuf {
    directory.0.join(format!(
        "projection-sender.json.otter-recovery-00000000-0000-0000-0000-{index:012x}.json"
    ))
}
#[test]
fn unique_valid_recovery_is_restored_without_deleting_it_ambiguity_and_corruption_fail_closed() {
    let (directory, store) = setup();
    prepare(&store, &directory, &selection(1)).unwrap();
    let bytes = std::fs::read(directory.path()).unwrap();
    drop(store);
    std::fs::remove_file(directory.path()).unwrap();
    let copy = recovery(&directory, 1);
    std::fs::write(&copy, &bytes).unwrap();
    let reopened = directory.store();
    assert_eq!(reopened.pending(&AtomicBool::new(false)).unwrap().len(), 1);
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    assert_eq!(std::fs::read(&copy).unwrap(), bytes);
    drop(reopened);
    std::fs::remove_file(directory.path()).unwrap();
    std::fs::write(recovery(&directory, 2), &bytes).unwrap();
    assert_eq!(
        Store::open(&directory.0, &AtomicBool::new(false)).err(),
        Some(Error::Read)
    );
    assert!(!directory.path().exists());
    assert_eq!(std::fs::read(&copy).unwrap(), bytes);
    let bad = Directory::new();
    std::fs::write(recovery(&bad, 1), b"broken").unwrap();
    assert_eq!(
        Store::open(&bad.0, &AtomicBool::new(false)).err(),
        Some(Error::Format)
    );
    assert!(!bad.path().exists());
}
#[test]
fn malformed_duplicate_unknown_fields_digest_and_snapshot_are_rejected_as_a_whole() {
    let (directory, store) = setup();
    prepare(&store, &directory, &selection(1)).unwrap();
    let bytes = std::fs::read(directory.path()).unwrap();
    let good: Value = serde_json::from_slice(&bytes).unwrap();
    for (path, value) in [
        ("/version", json!(2)),
        ("/entries/0/cancelled", Value::Null),
        ("/entries/0/selection/scope", json!("bad")),
        ("/entries/0/selection/unit", json!("other")),
        ("/entries/0/envelope/content/digest", json!("b".repeat(64))),
        ("/entries/0/snapshot/imageBase64", json!("not-base64")),
        ("/entries/0/snapshot/width", json!(3)),
        ("/entries/0/envelope/source/revision", json!(2)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            Document::decode(&serde_json::to_vec(&bad).unwrap(), &AtomicBool::new(false)).is_err(),
            "{path}"
        );
    }
    let mut duplicated = good.clone();
    duplicated["entries"]
        .as_array_mut()
        .unwrap()
        .push(good["entries"][0].clone());
    assert!(Document::decode(
        &serde_json::to_vec(&duplicated).unwrap(),
        &AtomicBool::new(false)
    )
    .is_err());
    for path in [
        "",
        "/entries/0",
        "/entries/0/selection",
        "/entries/0/snapshot",
    ] {
        let mut bad = good.clone();
        bad.pointer_mut(path).unwrap()["token"] = json!("never");
        assert!(
            Document::decode(&serde_json::to_vec(&bad).unwrap(), &AtomicBool::new(false)).is_err()
        );
    }
    assert_eq!(
        Document::decode(
            br#"{"version":1,"version":1,"entries":[]}"#,
            &AtomicBool::new(false)
        )
        .err(),
        Some(Error::Format)
    );
}
#[test]
fn tampered_valid_signature_shape_is_not_accepted_for_retry() {
    let (directory, store) = setup();
    prepare(&store, &directory, &selection(1)).unwrap();
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    drop(store);
    value["entries"][0]["envelope"]["nonce"] = json!("3".repeat(32));
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(directory.path(), &bytes).unwrap();
    let reopened = directory.store();
    assert_eq!(
        prepare(&reopened, &directory, &selection(1)).err(),
        Some(Error::Identity)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
}
#[test]
fn full_count_and_oversized_disk_documents_preserve_existing_data() {
    let (directory, store) = setup();
    for index in 0..MAX_ENTRIES {
        prepare(&store, &directory, &selection(index)).unwrap();
    }
    let bytes = std::fs::read(directory.path()).unwrap();
    assert_eq!(
        prepare(&store, &directory, &selection(MAX_ENTRIES)).err(),
        Some(Error::Limit)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    assert_eq!(
        store.pending(&AtomicBool::new(false)).unwrap().len(),
        MAX_ENTRIES
    );
    let large = Directory::new();
    let file = std::fs::File::create(large.path()).unwrap();
    file.set_len((MAX_BYTES + 1) as u64).unwrap();
    drop(file);
    assert_eq!(
        Store::open(&large.0, &AtomicBool::new(false)).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        std::fs::metadata(large.path()).unwrap().len(),
        (MAX_BYTES + 1) as u64
    );
}
#[test]
fn bounded_wire_rejects_entry_65_and_large_metadata_before_png_materialization() {
    let (directory, store) = setup();
    prepare(&store, &directory, &selection(1)).unwrap();
    let good: Value = serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    let mut many = good.clone();
    many["entries"] = Value::Array(vec![good["entries"][0].clone(); MAX_ENTRIES + 1]);
    assert_eq!(
        Document::decode(&serde_json::to_vec(&many).unwrap(), &AtomicBool::new(false)).err(),
        Some(Error::Format)
    );
    let mut large = good.clone();
    large["entries"][0]["envelope"]["nonce"] = json!("a".repeat(8193));
    assert_eq!(
        Document::decode(
            &serde_json::to_vec(&large).unwrap(),
            &AtomicBool::new(false)
        )
        .err(),
        Some(Error::Limit)
    );
    let mut bad_key = good;
    bad_key["entries"][0]["selection"]["scope"] = json!("a".repeat(1024 * 1024));
    assert_eq!(
        Document::decode(
            &serde_json::to_vec(&bad_key).unwrap(),
            &AtomicBool::new(false)
        )
        .err(),
        Some(Error::Format)
    );
}
#[test]
fn total_48_mib_gate_rejects_append_before_commit_and_keeps_previous_journal() {
    let (directory, store) = setup();
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
        // 合法的未知 ancillary chunk；压缩字节接近 4 MiB，像素仍仅 2×1。
        writer
            .write_chunk(
                png::chunk::ChunkType(*b"npAD"),
                &vec![0; super::super::super::accept::MAX_PNG - 1024],
            )
            .unwrap();
        writer.finish().unwrap();
    }
    let png: Pixels = png.into();
    for index in 0..12 {
        let previous = std::fs::read(directory.path()).ok();
        let result = store.prepare(
            Preparation {
                selection: &selection(index),
                frame: Frame {
                    png: png.clone(),
                    ..frame()
                },
                identity_root: &directory.identity(),
                device: "device-1",
                now_ms: 1_000,
            },
            &AtomicBool::new(false),
        );
        if matches!(&result, Err(Error::Limit)) {
            assert_eq!(std::fs::read(directory.path()).ok(), previous);
            assert!((8..=9).contains(&index));
            assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), index);
            assert!(previous.unwrap().len() <= MAX_BYTES);
            return;
        }
        assert!(result.is_ok());
    }
    panic!("48 MiB sender journal gate did not reject append");
}
