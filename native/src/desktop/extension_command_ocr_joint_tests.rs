//! Explicit hidden native OCR button through the real, isolated Loom extension bridge.
use super::*;
use std::{io::Read, path::PathBuf, sync::atomic::AtomicBool};

#[path = "extension_command_ocr_cancel_probe.rs"]
mod cancel_probe;
#[path = "extension_command_translation_joint.rs"]
mod translation_joint;

#[test]
#[ignore = "requires the signed OCR plugin in an isolated Loom daemon and a synthetic PNG"]
fn real_loom_ocr_button_uploads_and_commits_attachment() {
    let input = PathBuf::from(std::env::var_os("OTTER_OCR_JOINT_INPUT").unwrap());
    assert!(input.is_absolute());
    let mut raw = Vec::new();
    std::fs::File::open(&input)
        .unwrap()
        .take(4097)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 4096);
    let data: Value = serde_json::from_slice(&raw).unwrap();
    let manifest = PathBuf::from(data["manifest"].as_str().unwrap());
    let image = PathBuf::from(data["image"].as_str().unwrap());
    assert!(manifest.is_absolute() && image.is_absolute());
    let expected = data["expectedText"].as_str().unwrap();
    let identity = crate::loom_discovery::Config::from_lookup(|name| {
        (name == "LOOM_MANIFEST_PATH").then(|| manifest.clone().into_os_string())
    })
    .load()
    .unwrap();
    let cancel = AtomicBool::new(false);
    let directory = thread::spawn({
        let identity = identity.clone();
        move || {
            let session = crate::loom_extension_session::Session::open(&identity, &cancel).unwrap();
            session.snapshot_handle().unwrap()
        }
    })
    .join()
    .unwrap();
    unsafe {
        let mut fixture = Fixture::with_image(directory.clone(), || {
            crate::image_decode::Image::open(&image).unwrap()
        });
        let stamp = fixture.state().canvas.borrow().editor.document_stamp();
        let original = fixture.state().extension_document.borrow().store.stamp();
        let discarded = super::super::super::clipboard::capture_extension_copy(|| {
            fixture.begin_button(false, identity.clone());
            // Stop at the real model's Apply packet, not at the earlier input/image requests.
            let end = Instant::now() + Duration::from_secs(12);
            loop {
                let pending = fixture
                    .owner
                    .pending
                    .as_ref()
                    .expect("OCR ended before Apply");
                if pending.job.request_pending() {
                    if pending.job.apply_pending() {
                        break;
                    }
                    fixture.tick();
                }
                assert!(Instant::now() < end, "real OCR result did not reach Apply");
                thread::sleep(Duration::from_millis(2));
            }
            let flag = fixture
                .owner
                .pending
                .as_ref()
                .unwrap()
                .intent
                .cancel
                .clone();
            assert!(
                available(fixture.state(), false),
                "repeat OCR must remain reachable"
            );
            fixture.click_button(false);
            assert!(flag.load(Ordering::Acquire), "repeat OCR did not cancel");
            fixture.finish();
        });
        assert!(discarded.is_empty());
        assert!(original.matches(&fixture.state().extension_document.borrow().store));
        // Capture only the UTF-16 payload at the OS clipboard boundary, after the authority fence.
        let copies = super::super::super::clipboard::capture_extension_copy(|| {
            fixture.begin_button(false, identity.clone());
            fixture.finish();
        });
        let doc = fixture.state().extension_document.borrow();
        let attachment = doc
            .store
            .items()
            .iter()
            .find(|item| item.attachment.type_id() == "neuro.official/ocr.result.v1")
            .expect("real Loom OCR attachment not committed");
        let payload = attachment.attachment.payload().unwrap();
        assert_eq!(payload["fullText"], expected);
        assert_eq!(payload["sourceWidth"], data["width"]);
        assert_eq!(payload["sourceHeight"], data["height"]);
        assert_eq!(
            payload["textBlocks"].as_array().unwrap().len() as u64,
            data["blocks"].as_u64().unwrap()
        );
        let clipboard = format!("{}\0", expected.replace('\n', "\r\n"));
        assert_eq!(copies, [clipboard.encode_utf16().collect::<Vec<_>>()]);
        let canvas = fixture.state().canvas.borrow();
        assert!(canvas
            .extension_views
            .as_ref()
            .unwrap()
            .active(&canvas, None, false));
        assert!(stamp.matches(&canvas.editor));
    }
    if let Some(probe) = data.get("inflightCancel") {
        unsafe {
            cancel_probe::verify(probe, directory.clone(), identity.clone(), &image, expected)
        };
    }
    if data["translationLocal"] == true || data["translationAuto"] == true {
        let recovery = data["translationRecovery"].as_str().map(PathBuf::from);
        unsafe {
            translation_joint::verify(
                directory,
                identity,
                &image,
                expected,
                recovery.as_deref(),
                data["translationAuto"] == true,
                data["translationSafetyNotice"] == true,
            )
        };
    }
}
