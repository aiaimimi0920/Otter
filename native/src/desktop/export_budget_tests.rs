//! 独立预算验证 WIC 合成资源生命周期，不耗尽全局预算、不触碰磁盘。
use super::*;
use crate::{byte_budget::Budget, erase_layers::Layers};

fn fixture() -> (Image, Editor) {
    let image = Image {
        width: 4,
        height: 4,
        pixels: vec![255; 64].into(),
    };
    let mut editor = Editor::with_layers(Layers {
        size: (4, 4),
        base: image.pixels.clone(),
        annotation_size: Some((2, 1)),
        annotations: Some(vec![255; 8].into()),
    })
    .unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 2.0,
            height: 2.0,
        })
        .unwrap();
    (image, editor)
}

#[test]
fn composition_reserves_output_and_independent_uploads_then_retains_only_output() {
    let _apartment = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let (image, editor) = fixture();
    // 裁剪输出 16 + 完整底图上传 64 + 独立标注上传 8；不按底图尺寸误算标注。
    let budget = Budget::new(88);
    let peak = std::cell::Cell::new(0);
    let output = compose_with_budget(&image, &editor, &wic, |bytes| {
        let lease = budget.reserve(bytes);
        peak.set(peak.get().max(budget.used()));
        lease
    })
    .unwrap();
    assert_eq!(peak.get(), 88);
    assert_eq!(budget.used(), 16);
    let mut pixels = [0; 16];
    unsafe {
        output
            .bitmap
            .CopyPixels(std::ptr::null(), 8, &mut pixels)
            .unwrap();
    }
    assert_eq!(pixels, [255; 16]);
    drop(output);
    assert_eq!(budget.used(), 0);
}

#[test]
fn either_admission_failure_releases_all_leases_without_modifying_document() {
    let _apartment = Apartment::new().unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let (image, editor) = fixture();
    let stamp = editor.document_stamp();
    let source = editor.layers().unwrap().clone();
    for limit in [15, 87] {
        let budget = Budget::new(limit);
        let result = compose_with_budget(&image, &editor, &wic, |bytes| budget.reserve(bytes));
        match result {
            Err(error) => assert_eq!(error.code(), windows::Win32::Foundation::E_OUTOFMEMORY),
            Ok(_) => panic!("composition exceeded budget"),
        }
        assert_eq!(budget.used(), 0);
        assert!(stamp.matches(&editor));
        assert!(editor.layers().unwrap().same_storage(&source));
    }
}
