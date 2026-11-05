//! 原生 border-box 测量、内容排版与文字反馈回归。
use super::*;
#[test]
fn collapsed_text_content_box_does_not_reject_the_scene_or_create_draw_items() {
    for layout in [
        r#"{"width":0,"height":0}"#,
        r#"{"width":1,"height":1,"padding":3}"#,
    ] {
        let json = format!(
            r#"{{"id":"root","type":"text","layout":{layout},"style":{{"borderWidth":5}},"props":{{"text":"Visible after resize"}}}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [64., 64.])
                .unwrap();
        assert!(presentation.text().items().is_empty());
        assert_eq!(presentation.geometry().nodes().len(), 1);
    }
}
#[test]
fn container_class_width_fills_parent_even_without_stretch_alignment() {
    for kind in ["view", "row", "column", "stack", "scroll"] {
        let json = format!(
            r#"{{"id":"root","type":"column","layout":{{"padding":10,"align":"flex-start"}},"children":[{{"id":"child","type":"{kind}","layout":{{"height":10}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 100.])
                .unwrap();
        assert_eq!(presentation.geometry().nodes()[1].size()[0], 180.);
        assert_eq!(presentation.geometry().nodes()[1].origin(), [10., 10.]);
    }
}
#[test]
fn absolute_container_default_width_differs_from_explicit_auto() {
    for (width, expected) in [("", 100.), (r#", "width":"auto""#, 70.)] {
        let json = format!(
            r#"{{"id":"root","type":"view","layout":{{"position":"relative","height":100}},"children":[{{"id":"child","type":"view","layout":{{"position":"absolute","left":10,"right":20,"top":0,"height":10{width}}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 100.])
                .unwrap();
        assert_eq!(presentation.geometry().nodes()[1].size()[0], expected);
    }
}
#[test]
fn root_absolute_auto_uses_two_anchors_but_preserves_default_container_width() {
    for (width, expected) in [
        (r#", "width":"auto""#, 160.),
        ("", 200.),
        (r#", "width":50"#, 50.),
    ] {
        let json = format!(
            r#"{{"id":"root","type":"view","layout":{{"position":"absolute","left":10,"right":30,"top":5,"bottom":15{width}}}}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 100.])
                .unwrap();
        assert_eq!(presentation.geometry().nodes()[0].size(), [expected, 80.]);
        assert_eq!(presentation.geometry().nodes()[0].origin(), [10., 5.]);
    }
}
#[test]
fn root_auto_stretch_obeys_minimum_and_border_floor_after_anchor_subtraction() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"position":"absolute","left":40,"right":40,"top":40,"bottom":40,"width":"auto","minWidth":30,"maxHeight":1},"style":{"borderWidth":5}}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [64., 64.])
            .unwrap();
    assert_eq!(presentation.geometry().nodes()[0].size(), [30., 10.]);
}
#[test]
fn root_auto_height_tracks_content_and_reflow_without_changing_viewport() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"padding":4,"gap":3},"children":[{"id":"a","type":"view","layout":{"height":20}},{"id":"b","type":"view","layout":{"height":10}}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 100.])
            .unwrap();
    assert_eq!(presentation.geometry().nodes()[0].size(), [200., 41.]);
    assert_eq!(presentation.size(), [200., 100.]);
}
#[test]
fn root_positioning_uses_viewport_and_final_constrained_size() {
    for (position, expected) in [("relative", [-20., -10.]), ("absolute", [150., 70.])] {
        let json = format!(
            r#"{{"id":"root","type":"view","layout":{{"position":"{position}","right":"10%","bottom":"10%","width":5,"minWidth":30,"height":20}},"children":[{{"id":"child","type":"view","layout":{{"width":10,"height":10}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 100.])
                .unwrap();
        assert_eq!(presentation.geometry().nodes()[0].origin(), expected);
        assert_eq!(presentation.geometry().nodes()[1].origin(), expected);
        assert_eq!(presentation.size(), [200., 100.]);
    }
}
#[test]
fn root_start_offsets_win_and_use_computed_font() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"position":"absolute","left":"1em","right":0,"top":5,"bottom":0,"width":30,"height":20},"style":{"fontSize":24}}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 100.])
            .unwrap();
    assert_eq!(presentation.geometry().nodes()[0].origin(), [24., 5.]);
}
#[test]
fn root_explicit_constraints_do_not_resize_host_viewport() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"width":"50%","height":300,"maxHeight":150,"minWidth":120}}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 100.])
            .unwrap();
    assert_eq!(presentation.size(), [200., 100.]);
    let root = &presentation.geometry().nodes()[0];
    assert_eq!(root.size(), [120., 150.]);
    assert_eq!(
        root.clip(),
        crate::surface_clip::Rect {
            left: 0.,
            top: 0.,
            right: 120.,
            bottom: 100.
        }
    );
}
#[test]
fn root_border_floor_uses_computed_font_and_keeps_host_clip() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"width":1,"height":1,"maxWidth":2,"maxHeight":2},"style":{"fontSize":20,"borderWidth":"1em"}}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [10., 10.])
            .unwrap();
    let root = &presentation.geometry().nodes()[0];
    assert_eq!(root.size(), [40., 40.]);
    assert_eq!(root.clip().right, 10.);
}
#[test]
fn percent_padding_uses_containing_width_for_nested_layout_and_text() {
    let scene = Scene::parse(br#"{"id":"root","type":"row","layout":{"align":"flex-start"},"children":[{"id":"box","type":"column","layout":{"width":80,"height":100,"padding":"10%"},"children":[{"id":"fill","type":"view","layout":{"width":10,"height":10}}]},{"id":"text","type":"text","layout":{"width":80,"padding":"10%"},"props":{"text":"Text wraps in content"}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [200., 200.]).unwrap();
    let prepared = measure.prepare(&scene, [200., 200.], context()).unwrap();
    assert_eq!(prepared.tree().nodes()[1].containing_width, 200.);
    assert_eq!(prepared.tree().nodes()[2].origin, [20., 20.]);
    let text = &prepared.text().items()[0];
    assert_eq!(text.origin(), [100., 20.]);
    assert_eq!(text.allocated()[0], 40.);
    assert_eq!(
        prepared.tree().nodes()[3].size[1],
        text.layout().size()[1] + 40.
    );
}
#[test]
fn absolute_percent_padding_uses_parent_padding_box() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"position":"relative","padding":20},"style":{"borderWidth":10},"children":[{"id":"text","type":"text","layout":{"position":"absolute","left":0,"top":0,"width":80,"height":100,"padding":"10%"},"props":{"text":"Text"}}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [200., 200.])
            .unwrap();
    let text = &presentation.text().items()[0];
    assert_eq!(text.origin(), [28., 28.]);
    assert_eq!(text.allocated()[0], 44.);
}
#[test]
fn tiny_explicit_and_maximum_sizes_cannot_compress_border_and_padding() {
    for kind in ["row", "column", "stack"] {
        let json = format!(
            r#"{{"id":"root","type":"{kind}","children":[{{"id":"child","type":"view","layout":{{"width":1,"height":1,"maxWidth":2,"maxHeight":2,"padding":3}},"style":{{"borderWidth":5}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let mut measure = NativeMeasure::new(&scene, base(), context(), [10., 10.]).unwrap();
        let prepared = measure.prepare(&scene, [10., 10.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].size, [16., 16.]);
    }
}
#[test]
fn absolute_end_anchor_uses_noncompressible_box_size() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"position":"relative","height":10},"children":[{"id":"child","type":"view","layout":{"position":"absolute","right":0,"bottom":0,"width":1,"height":1,"padding":3},"style":{"borderWidth":5}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [10., 10.]).unwrap();
    let prepared = measure.prepare(&scene, [10., 10.], context()).unwrap();
    assert_eq!(prepared.tree().nodes()[1].size, [16., 16.]);
    assert_eq!(prepared.tree().nodes()[1].origin, [-6., -6.]);
}
#[test]
fn presentation_geometry_and_text_share_border_inset_ancestor_clip() {
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"position":"relative","height":64,"overflowX":"hidden","overflowY":"hidden"},"style":{"borderWidth":"0.5em","fontSize":20},"children":[{"id":"text","type":"text","layout":{"position":"absolute","left":0,"top":0,"width":100,"height":100},"props":{"text":"clipped"}}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [64., 64.])
            .unwrap();
    let expected = crate::surface_clip::Rect {
        left: 10.,
        top: 10.,
        right: 54.,
        bottom: 54.,
    };
    assert_eq!(presentation.geometry().nodes()[1].clip(), expected);
    assert_eq!(presentation.text().items()[0].clip(), expected);
    assert_eq!(presentation.geometry().nodes()[0].clip().left, 0.);
}

#[test]
fn absolute_child_anchors_to_padding_box_not_outer_border_or_content_box() {
    for kind in ["row", "stack"] {
        let json = format!(
            r#"{{"id":"root","type":"{kind}","layout":{{"position":"relative","padding":10,"height":100}},"style":{{"borderWidth":5}},"children":[{{"id":"child","type":"view","layout":{{"position":"absolute","left":0,"right":0,"top":0,"bottom":0}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 100.]).unwrap();
        let prepared = measure.prepare(&scene, [100., 100.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].origin, [5., 5.]);
        assert_eq!(prepared.tree().nodes()[1].size, [90., 90.]);
    }
}

#[test]
fn border_width_uses_css_length_rules_and_solid_default() {
    for (style, expected) in [
        ("{}", 0.),
        (r#"{"borderWidth":0}"#, 0.),
        (r#"{"borderWidth":"1em"}"#, 16.),
        (r#"{"borderWidth":"2cqw"}"#, 8.),
        (r#"{"borderWidth":"50%"}"#, 3.),
        (r#"{"borderWidth":"auto"}"#, 3.),
    ] {
        let json = format!(r#"{{"id":"root","type":"view","style":{style}}}"#);
        let scene = Scene::parse(json.as_bytes()).unwrap();
        assert_eq!(
            crate::surface_box::border(scene.root(), context()),
            expected
        );
    }
}

#[test]
fn border_and_padding_inset_flow_and_stack_children() {
    for kind in ["row", "column", "stack"] {
        let json = format!(
            r#"{{"id":"root","type":"{kind}","layout":{{"padding":3}},"style":{{"borderWidth":5}},"children":[{{"id":"child","type":"view","layout":{{"width":20,"height":20}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 100.]).unwrap();
        let prepared = measure.prepare(&scene, [100., 100.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].origin, [8., 8.]);
        assert_eq!(prepared.tree().nodes()[1].size, [20., 20.]);
    }
}

#[test]
fn text_content_width_and_reflow_height_include_box_insets_once() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"text","type":"text","layout":{"width":80,"padding":3},"style":{"borderWidth":5},"props":{"text":"A wrapping line with several words"}},{"id":"tail","type":"view","layout":{"height":10}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 300.]).unwrap();
    let prepared = measure.prepare(&scene, [100., 300.], context()).unwrap();
    let text = &prepared.text().items()[0];
    assert_eq!(text.origin(), [8., 8.]);
    assert_eq!(text.allocated()[0], 64.);
    let height = text.layout().size()[1] + 16.;
    assert!((prepared.tree().nodes()[1].size[1] - height).abs() < 0.01);
    assert!((prepared.tree().nodes()[2].origin[1] - height).abs() < 0.01);
    assert!(prepared.passes() <= 8);
}
