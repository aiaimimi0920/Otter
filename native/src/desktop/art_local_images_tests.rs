//! 端口与资源ID分离，空输入不伪造图片；真实准备worker沿原预算编码。
use super::*;
use crate::{desktop::loom_input_job::Source, editor::Editor};
use std::time::{Duration, Instant};
#[test]
fn prepared_ports_use_separate_safe_resource_ids_and_exact_png_metadata() {
    let editor = Editor::default();
    let unit = UnitId::parse("images-test").unwrap();
    let sources = ["input_image", "mask/图像"]
        .into_iter()
        .map(|port| {
            Source::capture(
                port.into(),
                unit.clone(),
                2,
                1,
                vec![0, 0, 255, 255, 0, 255, 0, 255].into(),
                &editor,
            )
            .unwrap()
        })
        .collect();
    let mut job = Job::default();
    job.start(unit.clone(), &editor, sources).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let batch = loop {
        if let Some(batch) = job.poll() {
            break batch.unwrap();
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    };
    let content = content(&batch).unwrap();
    let _com = crate::desktop::Apartment::new().unwrap();
    use base64::{engine::general_purpose::STANDARD, Engine};
    for (index, port) in ["input_image", "mask/图像"].into_iter().enumerate() {
        let id = format!("art-input:{index}");
        assert_eq!(
            content.snapshot["state"]["inputs"][port],
            json!({
                "kind":"image","resourceId":id,"mimeType":"image/png","width":2,"height":1
            })
        );
        let uri = content.resources[&id].as_str().unwrap();
        let png = STANDARD
            .decode(uri.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap();
        let image = crate::image_decode::Image::from_encoded(&png).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.pixels.as_ref(), &[0, 0, 255, 255, 0, 255, 0, 255]);
    }
    assert!(batch
        .freshness()
        .unwrap()
        .is_current(|id, stamp| id == &unit && stamp.matches(&editor)));
}
#[test]
fn empty_input_is_explicit_and_does_not_invent_a_resource() {
    let batch =
        PreparedBatch::empty_for_test(UnitId::parse("empty-art").unwrap(), &Editor::default());
    let content = content(&batch).unwrap();
    assert_eq!(content.snapshot["state"]["inputs"], json!({}));
    assert_eq!(content.resources, json!({}));
}
