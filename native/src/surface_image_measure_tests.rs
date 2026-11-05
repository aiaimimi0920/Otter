//! 已解码自然尺寸进入生产布局；数值来自旧组件及实际 Chromium 基线。
use super::*;
#[test]
fn inline_image_unavailable_has_layout_without_fake_natural_pixels() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"image","type":"image","props":{"src":"data:image/png,bad","alt":"unavailable"}}]}"#).unwrap().with_image_availability(|_| false);
    let mut measure = NativeMeasure::new(&scene, base(), context(), [300., 180.]).unwrap();
    let prepared = measure.prepare(&scene, [300., 180.], context()).unwrap();
    let image = prepared
        .tree()
        .nodes()
        .iter()
        .find(|p| p.node.id() == "image")
        .expect("inline img remains present even without decoded pixels");
    assert_eq!(image.size[0], 300.);
    assert!(image.size[1] >= 16. && image.size[1] < 64.);
}
fn sizes(root: serde_json::Value) -> Vec<[f32; 2]> {
    let scene = Scene::parse(&serde_json::to_vec(&root).unwrap()).unwrap();
    let images = [("image".into(), [80, 40]), ("second".into(), [80, 40])].into();
    let mut measure =
        NativeMeasure::with_images(&scene, base(), context(), [300., 180.], images).unwrap();
    let prepared = measure.prepare(&scene, [300., 180.], context()).unwrap();
    assert!(prepared.passes() <= 8);
    prepared
        .tree()
        .nodes()
        .iter()
        .filter(|p| p.node.kind() == Kind::Image)
        .map(|p| p.size)
        .collect()
}
#[test]
fn surface_image_fractional_logical_size_survives_measurement_and_reflow() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"alignItems":"start"},"children":[{"id":"image","type":"image","layout":{"width":25}}]}"#).unwrap();
    let mut measure = NativeMeasure::with_image_sizes(
        &scene,
        base(),
        context(),
        [300., 180.],
        [("image".into(), [2.5, 1.5])].into(),
    )
    .unwrap();
    for available in [[300., 180.], [40., 180.], [300., 180.]] {
        let prepared = measure.prepare(&scene, available, context()).unwrap();
        let image = prepared
            .tree()
            .nodes()
            .iter()
            .find(|p| p.node.id() == "image")
            .unwrap();
        assert_eq!(image.size, [25., 15.]);
    }
    for size in [
        [f32::NAN, 1.],
        [f32::from_bits(1); 2],
        [f32::INFINITY, 1.],
        [0., 1.],
        [-0.5, 1.],
        [16384.5, 1.],
    ] {
        assert!(NativeMeasure::with_image_sizes(
            &scene,
            base(),
            context(),
            [300., 180.],
            [("image".into(), size)].into()
        )
        .is_err());
    }
}
fn case(kind: &str, root: serde_json::Value, image: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"id":"root","type":kind,"layout":root,"children":[{"id":"image","type":"image","layout":image}]})
}
#[test]
fn surface_image_measure_matches_browser_column_baseline() {
    for (width, height, image, expected) in [
        (300, 180, serde_json::json!({}), [300., 150.]),
        (300, 180, serde_json::json!({"width":160}), [160., 80.]),
        (300, 180, serde_json::json!({"height":80}), [300., 80.]),
        (
            300,
            180,
            serde_json::json!({"width":120,"height":120}),
            [120., 120.],
        ),
        (40, 180, serde_json::json!({}), [40., 20.]),
        (300, 20, serde_json::json!({}), [300., 20.]),
        (
            300,
            180,
            serde_json::json!({"width":160,"padding":4}),
            [160., 84.],
        ),
    ] {
        assert_eq!(
            sizes(case(
                "column",
                serde_json::json!({"width":width,"height":height}),
                image
            )),
            vec![expected]
        );
    }
}
#[test]
fn surface_image_measure_matches_browser_row_and_auto_parent() {
    for (image, expected) in [
        (serde_json::json!({}), [300., 180.]),
        (serde_json::json!({"height":80}), [160., 80.]),
        (serde_json::json!({"width":160}), [160., 180.]),
        (serde_json::json!({"maxHeight":60}), [120., 60.]),
    ] {
        assert_eq!(
            sizes(case(
                "row",
                serde_json::json!({"width":300,"height":180}),
                image
            )),
            vec![expected]
        );
    }
    assert_eq!(
        sizes(case(
            "column",
            serde_json::json!({"width":300}),
            serde_json::json!({})
        )),
        vec![[300., 150.]]
    );
    for kind in ["row", "column"] {
        assert_eq!(
            sizes(case(
                kind,
                serde_json::json!({"width":300,"height":180,"align":"center"}),
                serde_json::json!({})
            )),
            vec![[80., 40.]]
        );
    }
}
#[test]
fn surface_image_measure_auto_minimum_does_not_squash_two_images() {
    let mut root = case(
        "column",
        serde_json::json!({"width":300,"height":180}),
        serde_json::json!({}),
    );
    root["children"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"id":"second","type":"image"}));
    assert_eq!(sizes(root), vec![[300., 150.], [300., 150.]]);
}
#[test]
fn surface_image_measure_absolute_percentages_use_padding_box() {
    for (mut image, expected) in [
        (serde_json::json!({"width":"50%"}), [150., 75.]),
        (serde_json::json!({"height":"50%"}), [180., 90.]),
        (
            serde_json::json!({"width":"50%","height":"50%"}),
            [150., 90.],
        ),
        (
            serde_json::json!({"width":"50%","padding":"10%"}),
            [150., 105.],
        ),
    ] {
        image["position"] = "absolute".into();
        image["left"] = 0.into();
        image["top"] = 0.into();
        assert_eq!(
            sizes(case(
                "column",
                serde_json::json!({"position":"relative","width":300,"height":180,"padding":20}),
                image
            )),
            vec![expected]
        );
    }
}
#[test]
fn surface_image_measure_requires_current_dimensions_and_resets_feedback() {
    let scene = Scene::parse(
        br#"{"id":"root","type":"column","children":[{"id":"image","type":"image"}]}"#,
    )
    .unwrap();
    assert!(NativeMeasure::new(&scene, base(), context(), [300., 180.]).is_err());
    for dimensions in [[0, 40], [80, 0], [16385, 1], [16384, 16384]] {
        assert!(NativeMeasure::with_images(
            &scene,
            base(),
            context(),
            [300., 180.],
            [("image".into(), dimensions)].into()
        )
        .is_err());
    }
    let mut measure = NativeMeasure::with_images(
        &scene,
        base(),
        context(),
        [300., 180.],
        [("image".into(), [80, 40])].into(),
    )
    .unwrap();
    for size in [[300., 180.], [40., 180.], [300., 180.]] {
        let prepared = measure.prepare(&scene, size, context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].size, [size[0], size[0] / 2.]);
    }
}
#[test]
fn surface_image_presentation_replacement_does_not_inherit_old_node_pixels() {
    use crate::{
        image_decode::Image, surface_image_paint::Bitmap, surface_presentation::Presentation,
    };
    let image = Image::from_surface(
        &vec![255; 80 * 40 * 4],
        "application/x-neuro-rgba8",
        Some((80, 40)),
        || false,
    )
    .unwrap();
    let weak = image.pixels.downgrade();
    let source = Bitmap::new(image).unwrap();
    let scene = || {
        Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"image","type":"image"}]}"#)
            .unwrap()
    };
    let mut p = Presentation::with_images(
        scene(),
        base(),
        context(),
        [300., 180.],
        [("image".into(), source)].into(),
    )
    .unwrap();
    assert_eq!(p.geometry().nodes()[1].size(), [300., 150.]);
    p.reflow(base(), context(), [40., 180.]).unwrap();
    assert_eq!(p.geometry().nodes()[1].size(), [40., 20.]);
    // 候选缺资源必须失败并保留旧帧；不能静默按相同 ID 使用旧图。
    assert!(p.replace(scene(), base(), context(), [300., 180.]).is_err());
    assert_eq!(p.geometry().nodes()[1].size(), [40., 20.]);
    assert!(weak.upgrade().is_some());
    let empty = Scene::parse(br#"{"id":"empty","type":"view"}"#).unwrap();
    p.replace(empty, base(), context(), [300., 180.]).unwrap();
    assert!(weak.upgrade().is_none());
}
#[test]
fn surface_image_presentation_scroll_preserves_geometry_and_image_owner() {
    use crate::{
        image_decode::Image, surface_image_paint::Bitmap, surface_presentation::Presentation,
    };
    let image = Image::from_surface(
        &vec![255; 80 * 40 * 4],
        "application/x-neuro-rgba8",
        Some((80, 40)),
        || false,
    )
    .unwrap();
    let weak = image.pixels.downgrade();
    let source = Bitmap::new(image).unwrap();
    let scene=Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":300,"height":180},"children":[{"id":"image","type":"image"},{"id":"second","type":"image"}]}"#).unwrap();
    let mut p = Presentation::with_images(
        scene,
        base(),
        context(),
        [300., 180.],
        [("image".into(), source.clone()), ("second".into(), source)].into(),
    )
    .unwrap();
    assert!(p.set_scrollbar("root", 1, 60.).unwrap());
    assert_eq!(p.geometry().nodes()[1].size(), [300., 150.]);
    assert_eq!(p.geometry().nodes()[1].origin(), [0., -60.]);
    assert!(weak.upgrade().is_some());
    drop(p);
    assert!(weak.upgrade().is_none());
}
#[test]
fn surface_image_refresh_preserves_unchanged_text_selection() {
    use crate::{
        image_decode::Image, surface_image_paint::Bitmap, surface_presentation::Presentation,
    };
    let scene = || {
        Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"text","type":"text","props":{"text":"Selected text","selectable":true}},{"id":"image","type":"image"}]}"#).unwrap()
    };
    let mut p = Presentation::new(
        scene().with_image_availability(|_| false),
        base(),
        context(),
        [300., 180.],
    )
    .unwrap();
    assert!(p.begin_text_selection([1., 5.]).unwrap());
    p.finish_text_selection([80., 5.]).unwrap();
    let selected = p.selected_text().unwrap();
    assert!(selected.is_some());
    let bitmap = Bitmap::new(Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    })
    .unwrap();
    p.refresh_images(
        scene(),
        base(),
        context(),
        [300., 180.],
        [("image".into(), bitmap)].into(),
    )
    .unwrap();
    assert_eq!(p.selected_text().unwrap(), selected);
    p.refresh_images(
        scene().with_image_availability(|_| false),
        base(),
        context(),
        [300., 180.],
        Default::default(),
    )
    .unwrap();
    assert_eq!(p.selected_text().unwrap(), selected);
}
#[test]
fn inline_image_placeholder_refresh_preserves_cross_node_selection_without_alt() {
    use crate::{
        image_decode::Image, surface_image_paint::Bitmap, surface_presentation::Presentation,
    };
    let scene = || {
        Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"first","type":"text","props":{"text":"First selected","selectable":true}},{"id":"image","type":"image","props":{"src":"data:image/png,bad","alt":"NOT SELECTABLE"},"layout":{"width":40,"height":32}},{"id":"last","type":"text","props":{"text":"Last selected","selectable":true}}]}"#).unwrap()
    };
    let mut frame = Presentation::new(
        scene().with_image_availability(|_| false),
        base(),
        context(),
        [300., 180.],
    )
    .unwrap();
    assert!(frame.begin_text_selection([1., 5.]).unwrap());
    let end = frame.text().items().last().unwrap().origin();
    assert!(frame
        .finish_text_selection([end[0] + 60., end[1] + 5.])
        .unwrap());
    assert!(frame.text_selection_keyboard_ready("first", true));
    // selected_text 只导出单节点范围；跨节点范围没有借此扩大剪贴板能力。
    assert!(frame.selected_text().unwrap().is_none());
    assert!(!frame
        .text()
        .items()
        .iter()
        .find(|item| item.node_id() == "image")
        .unwrap()
        .selectable());
    let bitmap = Bitmap::new(Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    })
    .unwrap();
    frame
        .refresh_images(
            scene().with_image_availability(|_| true),
            base(),
            context(),
            [300., 180.],
            [("image".into(), bitmap)].into(),
        )
        .unwrap();
    assert!(frame.text_selection_keyboard_ready("first", true));
    frame
        .refresh_images(
            scene().with_image_availability(|_| false),
            base(),
            context(),
            [300., 180.],
            Default::default(),
        )
        .unwrap();
    assert!(frame.text_selection_keyboard_ready("first", true));
}
