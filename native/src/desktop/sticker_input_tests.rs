//! 真实文件/解码与共享owner；只在linshi创建唯一测试文件，不使用用户数据或全局输入。
use super::*;
use crate::desktop::{configuration_stamp, image::Image, Apartment, State};
use std::{path::PathBuf, sync::atomic::Ordering};
pub(in crate::desktop) fn asset() -> Arc<Asset> {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 2,
        height: 2,
        pixels: [0, 0, 255, 255].repeat(4).into(),
    };
    let bytes = crate::desktop::export::png_raw(&image, &AtomicBool::new(false)).unwrap();
    let uri = crate::loom_base64::encode_png_data_url(&bytes, &AtomicBool::new(false)).unwrap();
    Asset::restore(uri.as_str(), "中文图片.png", &AtomicBool::new(false)).unwrap()
}
struct File(PathBuf);
impl File {
    fn new(bytes: &[u8]) -> Self {
        let path = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-sticker-{}.png",
            crate::desktop::document_state::new_unit_id()
                .unwrap()
                .as_str()
        ));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
#[test]
fn actual_file_loader_validates_pixels_type_capacity_and_cancellation() {
    let original = asset();
    let (_, encoded) = crate::sticker_image_params::image_parts(&original.uri).unwrap();
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let file = File::new(&bytes);
    let cancel = AtomicBool::new(false);
    let loaded = Asset::load(&file.0, &cancel).unwrap();
    assert_eq!((loaded.image.width, loaded.image.height), (2, 2));
    assert_eq!(loaded.image.pixels, original.image.pixels);
    assert_eq!(loaded.uri, original.uri);
    cancel.store(true, Ordering::Release);
    assert_eq!(
        Asset::load(&file.0, &cancel).err().unwrap().code(),
        windows::Win32::Foundation::E_ABORT
    );
    for bytes in [b"not a png".as_slice(), b"", b"GIF89a"] {
        let file = File::new(bytes);
        assert!(Asset::load(&file.0, &AtomicBool::new(false)).is_err());
    }
    let file = File::new(b"x");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&file.0)
        .unwrap()
        .set_len((crate::sticker_image_params::MAX_FILE_BYTES + 1) as u64)
        .unwrap();
    assert!(Asset::load(&file.0, &AtomicBool::new(false)).is_err());
}
#[test]
fn webp_and_bmp_are_real_decodable_uploads_not_merely_extension_filters() {
    use base64::Engine;
    let webp = Image::webp_for_test();
    let uri = format!(
        "data:image/webp;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(webp)
    );
    let loaded = Asset::restore(&uri, "image.webp", &AtomicBool::new(false)).unwrap();
    assert!(loaded.image.width > 0 && loaded.image.height > 0);
    let mut bmp = vec![0u8; 70];
    bmp[..2].copy_from_slice(b"BM");
    bmp[2..6].copy_from_slice(&70u32.to_le_bytes());
    bmp[10..14].copy_from_slice(&54u32.to_le_bytes());
    bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
    bmp[18..22].copy_from_slice(&2u32.to_le_bytes());
    bmp[22..26].copy_from_slice(&2u32.to_le_bytes());
    bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
    bmp[28..30].copy_from_slice(&24u16.to_le_bytes());
    for start in [54, 62] {
        bmp[start..start + 6].copy_from_slice(&[0, 0, 255, 0, 0, 255]);
    }
    let uri = format!(
        "data:image/x-ms-bmp;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bmp)
    );
    let loaded = Asset::restore(&uri, "image.bmp", &AtomicBool::new(false)).unwrap();
    assert_eq!(loaded.image.pixels, [0, 0, 255, 255].repeat(4).into());
    assert!(loaded.uri.starts_with("data:image/bmp;"));
}
#[test]
fn clear_and_set_same_asset_never_revive_old_configuration_eligibility() {
    let state = State::new(None, false).unwrap();
    let image = asset();
    let original = configuration_stamp::Stamp::capture(&state);
    state.sticker_input.set_asset(Some(image.clone())).unwrap();
    assert!(!original.matches(&state));
    let loaded = configuration_stamp::Stamp::capture(&state);
    state.sticker_input.set_asset(None).unwrap();
    state.sticker_input.set_asset(Some(image)).unwrap();
    assert!(!loaded.matches(&state));
    assert!(state.sticker_input.take_changed());
    assert!(!state.sticker_input.take_changed());
}
#[test]
fn restore_and_clear_preserve_unknown_fields_and_disabled_state_without_path_authority() {
    let image = asset();
    let raw=serde_json::value::RawValue::from_string(serde_json::json!({"image_path":image.uri,"image_filename":"C:/metadata/not-a-permission.png","image":crate::graph_params::DISABLED,"future":{"keep":3}}).to_string()).unwrap();
    let config = Config::restore(Some(&raw), &AtomicBool::new(false)).unwrap();
    assert!(config.disabled);
    assert_eq!(config.parameters()["future"]["keep"], 3);
    let control = Control::default();
    control.restore(config);
    control.set_asset(None).unwrap();
    let cleared = control.snapshot().parameters();
    assert!(cleared.get("image_path").is_none());
    assert!(cleared.get("image_filename").is_none());
    assert_eq!(cleared["image"], crate::graph_params::DISABLED);
    control.toggle().unwrap();
    assert!(control.snapshot().parameters().get("image").is_none());
    let raw = serde_json::value::RawValue::from_string(
        serde_json::json!({"image_path":"file:///C:/private.png"}).to_string(),
    )
    .unwrap();
    assert!(Config::restore(Some(&raw), &AtomicBool::new(false)).is_err());
}

#[test]
fn legal_full_unknown_parameters_round_trip_and_rejected_edits_are_atomic() {
    let count: Map<String, Value> = (0..128)
        .map(|i| (format!("future{i}"), Value::Null))
        .collect();
    let bytes: Map<String, Value> = [(
        "x".into(),
        Value::String("x".repeat(crate::loom_parameter_state::MAX_BYTES - 5)),
    )]
    .into_iter()
    .collect();
    for values in [count, bytes] {
        let raw = serde_json::value::RawValue::from_string(serde_json::to_string(&values).unwrap())
            .unwrap();
        let control = Control::default();
        control.restore(Config::restore(Some(&raw), &AtomicBool::new(false)).unwrap());
        let before = control.snapshot();
        assert_eq!(before.parameters(), values);
        assert!(control.toggle().is_err());
        assert!(control.set_asset(Some(asset())).is_err());
        assert!(before.stamp() == control.snapshot().stamp());
        assert!(!control.take_changed());
        assert_eq!(control.snapshot().parameters(), values);
    }
}
