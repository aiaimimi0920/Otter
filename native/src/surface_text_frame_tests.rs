use super::*;
#[test]
fn selectable_text_hit_and_highlight_follow_projected_origin_and_clip() {
    let scene = Scene::parse(br#"{"id":"root","type":"stack","layout":{"position":"relative","width":100,"height":80},"children":[{"id":"text","type":"text","props":{"text":"Native selectable text","selectable":true},"layout":{"position":"absolute","left":20,"top":10,"width":200,"height":40}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
    let prepared = measure.prepare(&scene, [100., 80.], context()).unwrap();
    let item = &prepared.text().items()[0];
    assert!(item.selectable());
    assert_eq!(item.origin(), [20., 10.]);
    let y = item.origin()[1] + item.layout().baseline() / 2.;
    assert_eq!(item.selection_start([20.1, y]).unwrap(), Some(0));
    assert_eq!(item.selection_start([10., y]).unwrap(), None);
    assert_eq!(item.selection_start([101., y]).unwrap(), None);
    assert_eq!(
        item.selection_position([1000., y]).unwrap(),
        item.layout().text_len()
    );
    let rects = item.selection_rects(0..item.layout().text_len()).unwrap();
    assert!(!rects.is_empty());
    assert!(rects
        .iter()
        .all(|r| r.left >= 20. && r.right <= 100. && r.top >= 10. && r.bottom <= 50.));
}

#[test]
fn selectable_is_strict_text_property_not_button_or_truthy_string() {
    for (kind, selectable) in [("text", "false"), ("text", "\"true\""), ("button", "true")] {
        let json = format!(
            r#"{{"id":"root","type":"{kind}","props":{{"text":"Text","selectable":{selectable}}},"layout":{{"width":100,"height":40}}}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
        let prepared = measure.prepare(&scene, [100., 80.], context()).unwrap();
        let item = &prepared.text().items()[0];
        assert!(!item.selectable());
        assert_eq!(item.selection_start([1., 1.]).unwrap(), None);
        assert!(item.selection_position([1., 1.]).is_err());
        assert!(item.selection_rects(0..1).is_err());
    }
}
#[test]
fn clipped_text_retains_layout_and_reflow_feedback() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"text","type":"text","props":{"text":"native text wrapping native text wrapping"},"layout":{"position":"relative","left":200,"width":50}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
    let prepared = measure.prepare(&scene, [100., 80.], context()).unwrap();
    let item = &prepared.text().items()[0];
    assert!(item.clip().empty());
    assert!(item.layout().line_count() > 1);
    assert!(item.layout().size()[1] > 0.);
    assert_eq!(item.origin()[0], 200.);
}
#[test]
fn final_frame_reflows_after_flex_shrink_and_keeps_overflow_explicit() {
    let scene=Scene::parse(br#"{"id":"root","type":"row","children":[{"id":"a","type":"text","props":{"text":"native text wrapping native text wrapping"},"layout":{"width":200,"height":20}},{"id":"b","type":"text","props":{"text":"native text wrapping native text wrapping"},"layout":{"width":200,"height":20}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
    let tree = Tree::build(&scene, [100., 80.], context(), &mut measure).unwrap();
    let frame = measure.text_frame(&tree).unwrap();
    assert_eq!(frame.items().len(), 2);
    assert_eq!(frame.items()[0].allocated(), [50., 20.]);
    assert!(frame.items()[0].layout().line_count() > 1);
    assert!(frame.items()[0].overflow()[1] > 0.);
    assert_eq!(frame.items()[1].origin(), [50., 0.]);
}
#[test]
fn final_frame_skips_zero_area_and_rejects_foreign_scene_tree() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"a","type":"text","props":{"text":"text"},"layout":{"height":0}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
    let tree = Tree::build(&scene, [100., 80.], context(), &mut measure).unwrap();
    assert!(measure.text_frame(&tree).unwrap().items().is_empty());
    let other = Scene::parse(br#"{"id":"root","type":"column"}"#).unwrap();
    let mut other_measure = NativeMeasure::new(&other, base(), context(), [100., 80.]).unwrap();
    let foreign = Tree::build(&other, [100., 80.], context(), &mut other_measure).unwrap();
    assert!(measure.text_frame(&foreign).is_err());
}
