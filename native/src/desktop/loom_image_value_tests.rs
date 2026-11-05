//! 真实 Shader PNG 经正式端口、下游 worker、保存快照保持原字节；预览不冒充正式输出。
use super::*;
use crate::{
    desktop::{
        loom_input_job::{Job, Source},
        loom_result_state::{Error as StateError, Results},
        shader_image::{self, tests as oracle},
    },
    graph_requests::Request,
    workspace_id::UnitId,
};
use base64::Engine;
use std::time::{Duration, Instant};

fn unit(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn request(generation: u64) -> Request {
    Request {
        id: unit("shader-request"),
        generation,
    }
}
fn prepared() -> shader_image::Prepared {
    shader_image::prepare(oracle::matrix(), &AtomicBool::new(false)).unwrap()
}
fn value() -> ImageValue {
    let output = prepared();
    ImageValue::from_png(output.image, output.png).unwrap()
}
#[test]
fn formal_png_preserves_straight_bytes_that_pbgra_reencoding_would_change() {
    let value = value();
    let original = value.encoded_png().unwrap();
    assert_eq!(
        oracle::hash(&oracle::decode_rgba(original, 256, 256)),
        oracle::STRAIGHT_SHA256
    );
    let encoded = value.png(&AtomicBool::new(false)).unwrap();
    assert!(original.same_storage(&encoded));
    let roundtrip = export::png_raw(value.image(), &AtomicBool::new(false)).unwrap();
    assert_ne!(
        oracle::hash(&oracle::decode_rgba(&roundtrip, 256, 256)),
        oracle::STRAIGHT_SHA256,
        "test must expose the real straight-RGB precision loss of PBGRA-only ownership"
    );
    assert_eq!(value.bytes(), value.pixels.len() + original.len());
    // 普通正式图片没有原 PNG；空编辑合成与直接 WIC 编码的解码像素必须兼容。
    let plain = ImageValue::from(value.image().clone());
    assert!(plain.encoded_png().is_none());
    let fallback = plain.png(&AtomicBool::new(false)).unwrap();
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (legacy, width, height) =
        export::png_payload(plain.image(), &crate::editor::Editor::default()).unwrap();
    assert_eq!((width, height), (256, 256));
    assert_eq!(
        oracle::decode_rgba(&fallback, width, height),
        oracle::decode_rgba(&legacy, width, height)
    );
    assert_eq!(plain.bytes(), plain.pixels.len());
}
#[test]
fn formal_aliases_share_png_and_pixels_and_reject_stale_or_cancelled_results() {
    let mut results = Results::new(unit("shader-node"));
    results.begin(request(1));
    results
        .preview(&request(1), 1, prepared().into_batch().unwrap())
        .unwrap();
    assert!(results.display().is_some());
    assert!(results.image_source("output_image").is_none());
    assert!(results.save_output_images().is_empty());
    let stamp = results.output_stamp();
    results
        .finish(&request(1), prepared().into_batch().unwrap())
        .unwrap();
    assert!(!stamp.matches(&results));
    let output = results.image_source("output").unwrap();
    let image = results.image_source("output_image").unwrap();
    assert!(output.image.pixels.same_storage(&image.image.pixels));
    assert!(output
        .image
        .encoded_png()
        .unwrap()
        .same_storage(image.image.encoded_png().unwrap()));
    let before = results.output_stamp();
    results.begin(request(2));
    assert_eq!(
        results.finish(&request(1), prepared().into_batch().unwrap()),
        Err(StateError::Stale)
    );
    assert!(before.matches(&results));
    assert!(results.cancel(&request(2)));
    assert_eq!(
        results.finish(&request(2), prepared().into_batch().unwrap()),
        Err(StateError::Stale)
    );
    assert!(before.matches(&results));
    let saved = results.save_output_images();
    assert_eq!(saved.len(), 2);
    assert!(saved.iter().all(|(_, value)| value
        .encoded_png()
        .unwrap()
        .same_storage(output.image.encoded_png().unwrap())));
}
#[test]
fn downstream_input_worker_transmits_original_png_not_a_second_wic_conversion() {
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let mut results = Results::new(unit("shader-node"));
    results.begin(request(1));
    let value = prepared();
    let png = value.png.clone();
    results
        .finish(&request(1), value.into_batch().unwrap())
        .unwrap();
    let input = Source::capture_output(
        "image".into(),
        unit("shader-node"),
        results.image_source("output_image").unwrap(),
    )
    .unwrap();
    let mut job = Job::default();
    let editor = crate::editor::Editor::default();
    job.start(unit("target"), &editor, vec![input]).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let batch = loop {
        if let Some(result) = job.poll() {
            break result.unwrap();
        }
        assert!(Instant::now() < deadline, "input worker did not finish");
        std::thread::sleep(Duration::from_millis(2));
    };
    let (
        _,
        crate::loom_art_wire::Input::InlinePng {
            base64,
            width,
            height,
        },
    ) = batch.descriptors().next().unwrap()
    else {
        panic!("PNG input required")
    };
    assert_eq!((width, height), (256, 256));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64)
        .unwrap();
    assert_eq!(bytes.as_slice(), png.as_ref());
    assert!(
        !batch.is_current(|_| Some(&editor)),
        "formal output still requires its own output stamp, not Editor-only validation"
    );
}
#[test]
fn inline_restore_preserves_original_png_and_shared_lifetime_without_external_access() {
    let value = value();
    let png = value.encoded_png().unwrap();
    let encoded = crate::loom_base64::encode_png_data_url(png, &AtomicBool::new(false)).unwrap();
    let restored = ImageValue::inline(encoded.as_str(), &AtomicBool::new(false)).unwrap();
    assert_eq!(
        restored.png(&AtomicBool::new(false)).unwrap().as_ref(),
        png.as_ref()
    );
    assert_eq!(restored.pixels, value.pixels);
    let weak = restored.encoded_png().unwrap().downgrade();
    let clone = restored.clone();
    drop(restored);
    assert!(weak.upgrade().is_some());
    drop(clone);
    assert!(weak.upgrade().is_none());
    for input in [
        "C:/private.png",
        "https://invalid.test/a.png",
        "data:image/png;base64,invalid",
    ] {
        assert!(ImageValue::inline(input, &AtomicBool::new(false)).is_err());
    }
    assert_eq!(
        ImageValue::inline(encoded.as_str(), &AtomicBool::new(true))
            .err()
            .unwrap()
            .code(),
        E_ABORT
    );
}
#[test]
fn cancelled_encoding_and_invalid_pairs_are_not_accepted_as_formal_png() {
    let value = value();
    assert_eq!(
        value.png(&AtomicBool::new(true)).unwrap_err().code(),
        E_ABORT
    );
    assert!(ImageValue::from_png(value.image.clone(), vec![0; 8].into()).is_err());
    let invalid = Image {
        width: 0,
        height: 1,
        pixels: vec![0; 4].into(),
    };
    assert!(ImageValue::from_png(invalid, value.png.unwrap()).is_err());
}
