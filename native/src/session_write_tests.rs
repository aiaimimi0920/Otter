use super::*;
const SOURCE:&[u8]=br#"{"documentSchemaVersion":1,"documentRevision":7,"extension":{"keep":true},"recycleBin":[{"opaque":"retained"}],"stickers":[{"id":"a","x":0,"y":0,"w":20,"h":20,"groupId":"g","src":"opaque","extension":"a"},{"id":"b","x":30,"y":0,"w":20,"h":20,"extension":"b"}],"links":[{"id":"edge","fromUnitId":"a","fromPortId":"out","toUnitId":"b","toPortId":"in","extension":"edge"}],"groups":[{"id":"g","name":"group","hidden":false,"locked":false,"extension":"group"}]}"#;
fn topology() -> Topology {
    Topology::capture(&decode(SOURCE).unwrap().workspace)
}
#[test]
fn existing_windows_need_only_the_final_output_pixel_budget() {
    let cancel = AtomicBool::new(false);
    let first = assemble(SOURCE, &topology(), &cancel, |_, old| Ok(old)).unwrap();
    let length = first.bytes.len();
    drop(first);
    // Independent budget fits only final output, without filling the process-shared pool, and remains safe for parallel tests.
    let budget = crate::byte_budget::Budget::new(length);
    let candidate = assemble_with_reserve(
        SOURCE,
        &topology(),
        &cancel,
        None,
        None,
        None,
        |_, old| Ok(WindowCandidate::new(old)),
        |size| budget.reserve(size),
    )
    .unwrap();
    assert_eq!(candidate.bytes.len(), length);
    assert_eq!(budget.used(), length);
    drop(candidate);
    assert_eq!(budget.used(), 0);
}
#[test]
fn maximum_sized_formatted_source_compacts_without_rebuilding_windows() {
    let mut original = br#"{"documentRevision":9,"stickers":[{"id":"a","x":0,"y":0,"w":20,"h":20}],"links":[],"extension":["#.to_vec();
    original.resize(MAX_SESSION_BYTES - 2, b' ');
    original.extend_from_slice(b"]}");
    let graph = Topology::capture(&decode(&original).unwrap().workspace);
    let mut builds = 0;
    let candidate = assemble(&original, &graph, &AtomicBool::new(false), |_, old| {
        builds += 1;
        Ok(old)
    })
    .unwrap();
    assert_eq!(builds, 1);
    assert!(candidate.bytes.len() < 256);
    assert_eq!(candidate.revision, 10);
    let value: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(value["extension"], json!([]));
}
#[test]
fn round_trip_retains_unknown_fields_and_advances_revision_once() {
    let cancel = AtomicBool::new(false);
    let mut graph = topology();
    graph.groups[0].name = "renamed".into();
    let candidate = assemble(SOURCE, &graph, &cancel, |sticker, mut window| {
        if sticker.id == "a" {
            window.insert("x".into(), json!(12.25));
        }
        Ok(window)
    })
    .unwrap();
    assert_eq!(candidate.expected_revision, 7);
    assert_eq!(candidate.revision, 8);
    let value: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(value["extension"], json!({"keep":true}));
    assert_eq!(value["recycleBin"], json!([{"opaque":"retained"}]));
    assert_eq!(value["stickers"][0]["src"], "opaque");
    assert_eq!(value["stickers"][0]["extension"], "a");
    assert_eq!(value["stickers"][0]["x"], 12.25);
    assert_eq!(value["links"][0]["extension"], "edge");
    assert_eq!(value["groups"][0]["extension"], "group");
    assert_eq!(value["groups"][0]["name"], "renamed");
    assert_eq!(decode(SOURCE).unwrap().revision, 7);
}
#[test]
fn escaped_root_keys_remain_valid_and_unknown_values_keep_their_raw_encoding() {
    let original = br#"{"documentRevision":7,"sti\u0063kers":[],"links":[],"plugin\"key":{"escaped":"\u4e2d","number":1.00}}"#;
    let graph = Topology::capture(&decode(original).unwrap().workspace);
    let candidate = assemble(original, &graph, &AtomicBool::new(false), |_, _| {
        unreachable!("empty workspace")
    })
    .unwrap();
    let root: BTreeMap<String, &RawValue> = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(root["stickers"].get(), "[]");
    assert_eq!(
        root["plugin\"key"].get(),
        r#"{"escaped":"\u4e2d","number":1.00}"#
    );
    assert_eq!(decode(&candidate.bytes).unwrap().revision, 8);
}
#[test]
fn verified_candidate_requires_exact_immutable_storage_and_both_revisions() {
    let mut candidate = assemble(SOURCE, &topology(), &AtomicBool::new(false), |_, old| {
        Ok(old)
    })
    .unwrap();
    assert!(is_verified(&candidate));
    candidate.expected_revision += 1;
    assert!(!is_verified(&candidate));
    candidate.expected_revision -= 1;
    candidate.revision += 1;
    assert!(!is_verified(&candidate));
    candidate.revision -= 1;
    let same_storage = candidate.bytes.clone();
    candidate.bytes = candidate.bytes.to_vec().into();
    assert!(!is_verified(&candidate));
    candidate.bytes = same_storage;
    assert!(is_verified(&candidate));
}
#[test]
fn deleted_objects_do_not_reappear_and_new_objects_do_not_inherit_extensions() {
    let mut graph = topology();
    graph.stickers.remove(0);
    graph.links.clear();
    graph.groups.clear();
    graph.stickers.push(Sticker {
        id: "new".into(),
        group_id: None,
    });
    let candidate = assemble(SOURCE, &graph, &AtomicBool::new(false), |sticker, old| {
        if sticker.id == "new" {
            assert!(old.is_empty());
            Ok(json!({"x":1,"y":2,"w":3,"h":4})
                .as_object()
                .unwrap()
                .clone())
        } else {
            Ok(old)
        }
    })
    .unwrap();
    let value: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(value["stickers"].as_array().unwrap().len(), 2);
    assert_eq!(value["stickers"][0]["id"], "b");
    assert_eq!(value["stickers"][0]["extension"], "b");
    assert!(value["stickers"][1].get("extension").is_none());
    assert!(value["links"].as_array().unwrap().is_empty());
    assert!(value["groups"].as_array().unwrap().is_empty());
}
#[test]
fn future_schema_exhausted_revision_invalid_topology_and_bad_window_never_deliver() {
    let graph = topology();
    let cancel = AtomicBool::new(false);
    let mut value: Value = serde_json::from_slice(SOURCE).unwrap();
    value["documentSchemaVersion"] = json!(2);
    assert!(matches!(
        assemble(value.to_string().as_bytes(), &graph, &cancel, |_, old| Ok(
            old
        )),
        Err(Error::Source)
    ));
    value["documentSchemaVersion"] = json!(1);
    value["documentRevision"] = json!(u64::MAX);
    assert!(matches!(
        assemble(value.to_string().as_bytes(), &graph, &cancel, |_, old| Ok(
            old
        )),
        Err(Error::Revision)
    ));
    let mut broken = graph.clone();
    broken.stickers.clear();
    assert!(matches!(
        assemble(SOURCE, &broken, &cancel, |_, old| Ok(old)),
        Err(Error::Topology)
    ));
    assert!(matches!(
        assemble(SOURCE, &graph, &cancel, |_, mut old| {
            old.insert("id".into(), "different".into());
            Ok(old)
        }),
        Err(Error::Window)
    ));
    assert!(matches!(
        assemble(SOURCE, &graph, &cancel, |_, mut old| {
            old.insert("w".into(), json!(0));
            Ok(old)
        }),
        Err(Error::Window)
    ));
}
#[test]
fn cancellation_before_and_during_build_returns_no_partial_candidate() {
    let graph = topology();
    let cancel = AtomicBool::new(true);
    assert!(matches!(
        assemble(SOURCE, &graph, &cancel, |_, _| panic!("cancelled callback")),
        Err(Error::Cancelled)
    ));
    cancel.store(false, Ordering::Release);
    let mut called = 0;
    let result = assemble(SOURCE, &graph, &cancel, |_, old| {
        called += 1;
        cancel.store(true, Ordering::Release);
        Ok(old)
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(called, 1);
}
#[test]
fn sink_enforces_budget_before_appending_and_observes_cancellation() {
    let cancel = AtomicBool::new(false);
    let mut sink = Sink::new(MAX_SESSION_BYTES, &cancel, crate::pixels::reserve).unwrap();
    sink.bytes.resize(MAX_SESSION_BYTES - 1, 0);
    assert!(sink.write_all(&[1, 2]).is_err());
    assert_eq!(sink.bytes.len(), MAX_SESSION_BYTES - 1);
    sink.write_all(&[1]).unwrap();
    assert!(sink.write_all(&[2]).is_err());
    let mut sink = Sink::new(1, &cancel, crate::pixels::reserve).unwrap();
    cancel.store(true, Ordering::Release);
    assert!(sink.write_all(&[1]).is_err());
    assert!(sink.bytes.is_empty());
}

#[test]
fn output_budget_follows_shared_bytes_and_failed_admission_releases_it() {
    let budget = crate::byte_budget::Budget::new(32);
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        Sink::new(33, &cancel, |size| budget.reserve(size)),
        Err(Error::Budget)
    ));
    assert_eq!(budget.used(), 0);
    let mut sink = Sink::new(32, &cancel, |size| budget.reserve(size)).unwrap();
    let pointer = sink.bytes.as_ptr();
    sink.write_all(&[1; 32]).unwrap();
    assert_eq!(pointer, sink.bytes.as_ptr());
    assert!(sink.write_all(&[2]).is_err());
    let bytes = crate::pixels::Pixels::from_reserved(sink.bytes, sink.lease);
    assert_eq!(pointer, bytes.as_ptr());
    let clone = bytes.clone();
    drop(bytes);
    assert_eq!(budget.used(), 32);
    drop(clone);
    assert_eq!(budget.used(), 0);
    assert!(matches!(
        Sink::new(32, &cancel, |size| {
            cancel.store(true, Ordering::Release);
            budget.reserve(size)
        }),
        Err(Error::Cancelled)
    ));
    assert_eq!(budget.used(), 0);
}
#[test]
fn checkpoint_revision_is_independent_without_changing_source_or_unknown_fields() {
    let original = br#"{"documentRevision":42,"stickers":[],"links":[],"extension":{"keep":true}}"#;
    let topology = Topology::capture(&crate::session_document::decode(original).unwrap().workspace);
    let cancel = AtomicBool::new(false);
    for revision in [0, 7, 99] {
        let candidate = assemble_owned_at_revision(
            original,
            &topology,
            &cancel,
            None,
            Some(revision),
            None,
            |_, _| unreachable!(),
        )
        .unwrap();
        assert_eq!(
            (candidate.expected_revision, candidate.revision),
            (revision, revision + 1)
        );
        let value: serde_json::Value = serde_json::from_slice(&candidate.bytes).unwrap();
        assert_eq!(value["extension"]["keep"], true);
        assert_eq!(
            crate::session_document::decode(original).unwrap().revision,
            42
        );
    }
    assert!(matches!(
        assemble_owned_at_revision(
            original,
            &topology,
            &cancel,
            None,
            Some(u64::MAX),
            None,
            |_, _| unreachable!()
        ),
        Err(Error::Revision)
    ));
}
