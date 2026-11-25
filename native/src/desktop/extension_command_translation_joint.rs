//! Real OCR-to-translation workflow through an isolated broker.
use super::*;

pub(super) unsafe fn verify(
    directory: Arc<Snapshot>,
    identity: crate::loom_bridge_identity::Identity,
    image: &std::path::Path,
    expected_source: &str,
    recovery: Option<&std::path::Path>,
    auto: bool,
    safety_notice: bool,
) {
    let mut fixture = Fixture::with_image(directory, || {
        crate::image_decode::Image::open(image).unwrap()
    });
    let stamp = fixture.state().canvas.borrow().editor.document_stamp();
    assert!(fixture
        .state()
        .extension_document
        .borrow()
        .store
        .items()
        .is_empty());
    assert!(
        !auto || recovery.is_none(),
        "recovery requires local-only mode"
    );
    let target = if safety_notice { "zh-CN" } else { "en" };
    let settings = crate::app_settings_document::Document::decode(
        &serde_json::to_vec(&json!({
            "schemaVersion": 1, "translationTargetLanguage": target,
            "translationProviderMode": if auto { "auto" } else { "local" },
        }))
        .unwrap(),
    )
    .unwrap();
    let run = |fixture: &mut Fixture, identity, cached: bool| {
        fixture.begin_button(true, identity);
        if cached {
            // The button authorizes the overlay command first; the worker falls
            // through to translation when no translated attachment can be toggled.
            assert_eq!(
                fixture.owner.pending.as_ref().unwrap().intent.command,
                crate::desktop::extension_command_route::TRANSLATION_OVERLAY
            );
        }
        let deadline = Instant::now() + Duration::from_secs(70);
        while fixture.owner.pending.is_some() {
            fixture
                .owner
                .process_with(&fixture.documents, &settings, || panic!("unexpected retry"));
            assert!(
                Instant::now() < deadline,
                "real local translation did not finish"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert!(!busy(fixture.state()));
    };
    let notices_before = fixture
        .state()
        .extension_notices
        .borrow()
        .live_unscoped_count();
    let copies = super::super::super::super::clipboard::capture_extension_copy(|| {
        run(&mut fixture, identity.clone(), false);
    });
    let source_clipboard = format!("{}\0", expected_source.replace('\n', "\r\n"));
    assert_eq!(
        copies,
        [source_clipboard.encode_utf16().collect::<Vec<_>>()]
    );

    let retained = recovery.map(|gate| {
        assert!(gate.is_absolute() && gate.is_dir());
        let source = {
            let document = fixture.state().extension_document.borrow();
            let [item] = document.store.items() else {
                panic!("failure must leave only the OCR attachment");
            };
            assert_eq!(
                item.attachment.type_id(),
                crate::extension_visual::OCR_RESULT
            );
            assert_eq!(
                item.attachment.payload().unwrap()["fullText"],
                expected_source
            );
            (
                item.attachment.id().to_owned(),
                item.attachment.revision(),
                item.digest.clone(),
            )
        };
        {
            let canvas = fixture.state().canvas.borrow();
            let views = canvas.extension_views.as_ref().unwrap();
            assert!(views.active(&canvas, None, false));
            assert!(!views.active(&canvas, None, true));
            assert!(stamp.matches(&canvas.editor));
        }
        assert_eq!(
            fixture
                .state()
                .extension_notices
                .borrow()
                .live_unscoped_count(),
            notices_before + 1
        );
        // The controller may start inference only after the failure and retained state are observed.
        std::fs::File::options()
            .write(true)
            .create_new(true)
            .open(gate.join("failed"))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(45);
        while !gate.join("ready").is_file() {
            assert!(
                Instant::now() < deadline,
                "isolated translation backend did not recover"
            );
            thread::sleep(Duration::from_millis(20));
        }
        let copies = super::super::super::super::clipboard::capture_extension_copy(|| {
            run(&mut fixture, identity, true);
        });
        assert!(
            copies.is_empty(),
            "explicit retry unexpectedly copied fresh OCR"
        );
        source
    });

    let document = fixture.state().extension_document.borrow();
    let find = |kind| {
        document
            .store
            .items()
            .iter()
            .find(|item| item.attachment.type_id() == kind)
            .unwrap_or_else(|| {
                panic!(
                    "attachment {kind} missing; available={:?}",
                    document
                        .store
                        .items()
                        .iter()
                        .map(|i| i.attachment.type_id())
                        .collect::<Vec<_>>()
                )
            })
    };
    let ocr = find(crate::extension_visual::OCR_RESULT);
    let translation = find(crate::extension_visual::TRANSLATION);
    if let Some((id, revision, digest)) = retained {
        assert_eq!(
            (ocr.attachment.id(), ocr.attachment.revision(), &ocr.digest),
            (id.as_str(), revision, &digest)
        );
        println!("translation_joint_recovered_same_ocr=true");
    }
    let ocr_payload = ocr.attachment.payload().unwrap();
    let output = translation.attachment.payload().unwrap();
    assert_eq!(ocr_payload["fullText"], expected_source);
    assert_eq!(output["originalText"], expected_source);
    assert_eq!(output["translated"], true);
    assert_eq!(output["targetLanguage"], target);
    assert_eq!(
        output["sourceAttachment"]["attachmentId"],
        ocr.attachment.id()
    );
    assert_eq!(
        output["sourceAttachment"]["revision"],
        ocr.attachment.revision()
    );
    assert_eq!(
        output["sourceAttachment"]["digest"].as_str(),
        ocr.digest.as_deref()
    );
    for axis in ["sourceWidth", "sourceHeight"] {
        assert_eq!(output[axis].as_f64(), ocr_payload[axis].as_f64());
    }
    let translated = output["text"].as_str().unwrap();
    println!("translation_joint_result={}", json!(translated));
    let canvas = fixture.state().canvas.borrow();
    let views = canvas.extension_views.as_ref().unwrap();
    assert!(views.active(&canvas, None, true));
    assert!(!views.active(&canvas, None, false));
    assert!(stamp.matches(&canvas.editor));

    let blocks = output["textBlocks"].as_array().unwrap();
    let indices: Vec<_> = blocks.iter().map(|b| &b["sourceBlockIndices"]).collect();
    if safety_notice {
        let case: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/ocr-translation-safety-notice.json"
        ))
        .unwrap();
        let expected = case["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row[1].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(expected_source, expected);
        assert_eq!(target, case["targetLanguage"]);
        assert_eq!(
            indices,
            case["paragraphIndices"]
                .as_array()
                .unwrap()
                .iter()
                .collect::<Vec<_>>()
        );
        let first = blocks[0]["translatedText"].as_str().unwrap();
        let second = blocks[1]["translatedText"].as_str().unwrap();
        println!("translation_joint_paragraph_mapping={}", json!(indices));
        check_safety_notice_terms(first, second);
    } else {
        assert_eq!(indices, [&json!([0]), &json!([1])]);
        check_baseline_terms(translated);
    }
}

fn check_safety_notice_terms(first: &str, second: &str) {
    assert!(first.contains("30") && first.to_ascii_lowercase().contains("kpa"));
    assert!(first.contains("阀") && !first.contains("09:30"));
    // A matching clock substring alone is not sufficient: after, afternoon,
    // and who calls whom change what the notice tells the reader to do.
    assert!(second.contains("09:30") && second.contains("后"));
    assert!(!second.contains("下午") && !second.contains("晚上"));
    let compact = second.replace(' ', "");
    assert!(["给林打电话", "给林先生打电话", "致电林", "打电话给林"]
        .iter()
        .any(|term| compact.contains(term)));
}

fn check_baseline_terms(translated: &str) {
    let normalized = translated.to_ascii_lowercase();
    for term in [
        "paddle-ocr-rs",
        "rust",
        "onnx runtime",
        "paddle ocr",
        "image",
    ] {
        assert!(
            normalized.contains(term),
            "translation lost {term}: {translated}"
        );
    }
    assert!(!translated.contains("进行图片文字识别"));
}
