//! 独立常量核非等比、完整wire域和身份；预期不调用被测逆映射或普通session writer。
use super::{test_fixture::*, *};
use crate::projection_edit::{
    fixture::{DIGEST, PROJECTION, SESSION},
    Operation, Request,
};
use serde_json::json;

#[test]
fn nonuniform_fixture_resize_is_not_an_edit_and_inverse_scalar_does_not_inflate() {
    let before = view();
    let resized = before.resized(Size::new(240.0, 100.0).unwrap());
    let expected = json!({"id":"shape-a","type":"rect","zIndex":1.25,
        "style":{"color":"#123456","width":3},"x":20,"y":20,"w":60,"h":25});
    assert!(edit_equal(
        &resized.local_objects().unwrap()["shape-a"],
        &expected
    ));
    assert!(before.changes_to(&resized).unwrap().is_empty());
    assert!(resized
        .changes_to(&resized.edited(resized.local_objects().unwrap()).unwrap())
        .unwrap()
        .is_empty());
    let edited = move_x(&resized, "shape-a", 40.0);
    let changes = before.changes_to(&edited).unwrap();
    assert_eq!(changes.len(), 1);
    let expected = json!({"id":"shape-a","type":"rect","zIndex":1.25,
        "style":{"color":"#123456","width":2},"x":20,"y":20,"w":30,"h":25});
    assert!(edit_equal(&changes[0].value, &expected));
    assert_eq!(before.basis_objects()["shape-a"]["x"], 10);
}

#[test]
fn text_and_serial_anchors_scale_by_axis_while_fonts_and_radius_use_forward_mean() {
    let objects = Objects::from([
        (
            "text".into(),
            json!({"id":"text","type":"text","zIndex":-4.5,
            "style":{"color":"#10203080","width":2},"x":10,"y":20,"text":" 甲\r\n乙 ","rotation":36000,"fontFamily":""}),
        ),
        (
            "serial".into(),
            json!({"id":"serial","type":"serial","zIndex":-4.5,
            "style":{"color":"#102030","width":0,"cornerRadius":14},"x":10,"y":20,"text":""}),
        ),
    ]);
    let before = View::from_basis(&basis(), Size::new(120.0, 100.0).unwrap(), objects).unwrap();
    let resized = before.resized(Size::new(240.0, 100.0).unwrap());
    let local = resized.local_objects().unwrap();
    let text = &local["text"];
    assert_eq!(
        (text["x"].as_f64(), text["y"].as_f64()),
        (Some(20.0), Some(20.0))
    );
    assert_eq!(text["fontSize"], 27.0);
    assert_eq!(text["fontFamily"], "");
    assert_eq!(text["text"], " 甲\r\n乙 ");
    assert_eq!(text["rotation"], 36000);
    assert_eq!(local["serial"]["fontSize"], 24.0); // round(14*1.15)=16，再乘1.5。
    assert_eq!(local["serial"]["style"]["cornerRadius"], 21.0);
    assert_eq!(local["serial"]["style"]["width"], 0.0);
    let mut changed = local;
    changed.get_mut("text").unwrap()["x"] = 40.into();
    changed.get_mut("text").unwrap()["fontSize"] = 54.into();
    changed.get_mut("serial").unwrap()["y"] = 30.into();
    changed.get_mut("serial").unwrap()["style"]["cornerRadius"] = 21.75.into();
    let result = resized.edited(changed).unwrap();
    assert_eq!(result.basis_objects()["text"]["x"], 20.0);
    assert_eq!(result.basis_objects()["text"]["fontSize"], 36.0);
    assert_eq!(result.basis_objects()["serial"]["y"], 30.0);
    assert_eq!(
        result.basis_objects()["serial"]["style"]["cornerRadius"],
        14.5
    );
    assert_eq!(result.basis_objects()["serial"]["fontSize"], 16.0); // 不按新radius偷偷重算字体。
}

#[test]
fn legal_wire_extremes_and_degenerate_paths_are_not_clamped_or_discarded() {
    let mut objects = Objects::new();
    for kind in ["rect", "round-rect", "ellipse", "triangle", "polygon"] {
        let value = json!({"id":kind,"type":kind,"zIndex":-999999.25,"x":-1000000,"y":1000000,
            "w":0,"h":0,"rotation":-36000,"sides":64,"style":{"color":"transparent","width":0,
            "fill":"#00000000","secondaryFill":"rgba(1,2,3,0.25)","cornerRadius":8192,"opacity":0}});
        objects.insert(kind.into(), value);
    }
    for kind in ["line", "arrow", "polyline", "brush", "highlighter"] {
        objects.insert(
            kind.into(),
            json!({"id":kind,"type":kind,"zIndex":3,
            "style":{"color":"#abcdef","width":8192},"points":[{"x":4,"y":5}]}),
        );
    }
    objects.insert(
        "text".into(),
        json!({"id":"text","type":"text","zIndex":0,"style":{"color":"#000","width":0},
        "x":0,"y":0,"text":"\u{0000}\r\n  ","fontFamily":"界".repeat(128),"fontSize":8192}),
    );
    let before =
        View::from_basis(&basis(), Size::new(120.0, 100.0).unwrap(), objects.clone()).unwrap();
    let resized = before.resized(Size::new(240.0, 300.0).unwrap());
    let local = resized.local_objects().unwrap();
    assert_eq!(local["arrow"]["style"]["width"], 20480.0);
    assert_eq!(local["text"]["fontSize"], 20480.0);
    assert_eq!(local["polygon"]["sides"], 64);
    assert_eq!(local["rect"]["w"], 0.0);
    assert_eq!(resized.edited(local).unwrap().basis_objects(), &objects);
    for kind in ["line", "arrow"] {
        let mut local = before.local_objects().unwrap();
        local.get_mut(kind).unwrap()["points"] =
            json!([{"x":1,"y":2},{"x":3,"y":4},{"x":5,"y":6},{"x":5,"y":6}]);
        let edited = before.edited(local).unwrap();
        assert_eq!(
            edited.basis_objects()[kind]["points"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
    }
}

#[test]
fn repeated_frame_resizing_shares_basis_and_never_accumulates_scalar_error() {
    let before = view();
    let mut current = before.clone();
    for _ in 0..100 {
        current = current
            .resized(Size::new(240.0, 100.0).unwrap())
            .resized(Size::new(120.0, 300.0).unwrap());
        current = current.edited(current.local_objects().unwrap()).unwrap();
        assert_eq!(current.basis_objects(), before.basis_objects());
    }
    let original = before.resized(Size::new(120.0, 100.0).unwrap());
    assert!(Arc::ptr_eq(&before.objects, &original.objects));
    assert!(before.changes_to(&current).unwrap().is_empty());
}

#[test]
fn deleting_one_object_does_not_reindex_other_objects_or_change_fractional_ties() {
    let mut objects = objects();
    let mut b = rect("shape-b");
    b["zIndex"] = (-9.25).into();
    let mut c = rect("shape-c");
    c["zIndex"] = (-9.25).into();
    objects.extend([("shape-b".into(), b.clone()), ("shape-c".into(), c.clone())]);
    let before = View::from_basis(&basis(), Size::new(120.0, 100.0).unwrap(), objects).unwrap();
    let mut local = before.local_objects().unwrap();
    local.remove("shape-a");
    let current = before.edited(local).unwrap();
    assert_eq!(current.basis_objects()["shape-b"], b);
    assert_eq!(current.basis_objects()["shape-c"], c);
    assert_eq!(
        before.changes_to(&current).unwrap(),
        vec![Change {
            object_id: "shape-a".into(),
            value: Value::Null
        }]
    );
}

#[test]
fn unchanged_extreme_fields_keep_original_numbers_when_only_one_coordinate_is_edited() {
    let mut object = rect("shape-a");
    object["style"]["width"] = 8192.into();
    object["style"]["cornerRadius"] = 8192.into();
    object["w"] = 1_000_000.into();
    let before = View::from_basis(
        &basis(),
        Size::new(120.0, 100.0).unwrap(),
        Objects::from([("shape-a".into(), object.clone())]),
    )
    .unwrap();
    for (w, h) in [
        (0.125, 0.333),
        (123.123, 710.71),
        (999999.99, 0.0001),
        (0.0001, 999999.99),
    ] {
        let resized = before.resized(Size::new(w, h).unwrap());
        let edited = move_x(&resized, "shape-a", w / 4.0);
        for key in ["width", "cornerRadius"] {
            assert_eq!(
                edited.basis_objects()["shape-a"]["style"][key],
                object["style"][key]
            );
        }
        assert_eq!(edited.basis_objects()["shape-a"]["w"], 1_000_000);
    }
}

#[test]
fn edit_comparison_matches_numeric_precision_but_not_text_or_array_reordering() {
    assert!(edit_equal(
        &json!({"x":-0.0000005,"y":1}),
        &json!({"y":1.0000004,"x":0})
    ));
    assert!(!edit_equal(&json!(0), &json!(0.0000005)));
    assert!(!edit_equal(&json!([1, 2]), &json!([2, 1])));
    assert!(!edit_equal(&json!(" x "), &json!("x")));
    let before = view();
    let current = move_x(&before, "shape-a", 10.0000004);
    assert!(before.changes_to(&current).unwrap().is_empty());
    let enlarged = before.resized(Size::new(1_000_000.0, 100.0).unwrap());
    let x = enlarged.local_objects().unwrap()["shape-a"]["x"]
        .as_f64()
        .unwrap();
    let current = move_x(&enlarged, "shape-a", x + 0.0001);
    assert_eq!(before.changes_to(&current).unwrap().len(), 1);
}

#[test]
fn large_local_coordinates_do_not_merge_adjacent_representable_numeric_changes() {
    // 独立Node Math.round oracle：两者乘1e6分别为2^52+1和2^52+2，必须不同。
    assert!(!edit_equal(
        &json!(4503599627.370497_f64),
        &json!(4503599627.370498_f64)
    ));
    let mut object = rect("shape-a");
    object["x"] = 4503.599627370497_f64.into();
    let before = View::from_basis(
        &Basis::new(DIGEST, 1, 1).unwrap(),
        Size::new(1_000_000.0, 1.0).unwrap(),
        Objects::from([("shape-a".into(), object)]),
    )
    .unwrap();
    let edited = move_x(&before, "shape-a", 4503599627.370498_f64);
    assert_eq!(before.changes_to(&edited).unwrap().len(), 1);
}

#[test]
fn font_defaults_do_not_expand_valid_near_limit_wire_objects_or_create_noop_edits() {
    let mut object = json!({"id":"text","type":"text","zIndex":0,"x":10,"y":20,
        "text":"","style":{"color":"#000","width":1}});
    let count = (16 * 1024 - serde_json::to_vec(&object).unwrap().len()) / 6; // JSON每个控制字符编码为六字节。
    object["text"] = "\u{0001}".repeat(count).into();
    assert!(count <= 4096);
    let mut explicit = object.clone();
    explicit["fontSize"] = 18.into();
    assert!(serde_json::to_vec(&object).unwrap().len() <= 16 * 1024);
    assert!(serde_json::to_vec(&explicit).unwrap().len() > 16 * 1024);
    let originals = Objects::from([("text".into(), object)]);
    let before = View::from_basis(
        &basis(),
        Size::new(120.0, 100.0).unwrap(),
        originals.clone(),
    )
    .unwrap();
    let local = before.resized(Size::new(240.0, 100.0).unwrap());
    assert_eq!(local.local_objects().unwrap()["text"]["fontSize"], 27.0);
    let roundtrip = local.edited(local.local_objects().unwrap()).unwrap();
    assert_eq!(roundtrip.basis_objects(), &originals);
    assert!(before.changes_to(&roundtrip).unwrap().is_empty());
}

#[test]
fn invalid_basis_size_local_style_and_unknown_fields_fail_without_partial_publication() {
    for (w, h) in [
        (0.0, 1.0),
        (-1.0, 2.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
        (1_000_001.0, 2.0),
        (f64::from_bits(1), 1.0),
    ] {
        assert_eq!(Size::new(w, h), Err(Error::Size));
    }
    assert!(Basis::new(DIGEST, 8192, 8192).is_err());
    assert!(Basis::new("x", 1, 1).is_err());
    let before = view();
    for patch in [
        json!({"style":{"color":"#000","width":8193}}),
        json!({"showArrowHead":true}),
        json!({"style":{"color":"url(secret)","width":2}}),
        json!({"type":"blur"}),
        json!({"id":"different"}),
    ] {
        let mut local = before.local_objects().unwrap();
        local
            .get_mut("shape-a")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(before.edited(local).is_err());
        assert_eq!(before.basis_objects(), &objects());
    }
    let other = View::from_basis(
        &Basis::new(&"b".repeat(64), 120, 100).unwrap(),
        before.size(),
        objects(),
    )
    .unwrap();
    assert_eq!(
        before.changes_to(&other),
        Err(Error::Protocol(super::super::Error::Basis))
    );
}

#[test]
fn wire_object_point_and_total_budgets_are_unchanged_by_internal_local_coordinates() {
    let before = view();
    let mut many = Objects::new();
    for index in 0..257 {
        let id = format!("shape-{index}");
        many.insert(id.clone(), rect(&id));
    }
    assert_eq!(before.edited(many), Err(Error::Limit));
    let mut point = json!({"id":"line","type":"line","zIndex":0,"style":{"color":"#000","width":1},
        "points":vec![json!({"x":1,"y":2});2049]});
    assert!(before
        .edited(Objects::from([("line".into(), point.clone())]))
        .is_err());
    point["points"] = json!(vec![json!({"x":123456.123456,"y":654321.654321}); 1000]);
    assert!(View::from_basis(
        &basis(),
        before.size(),
        Objects::from([("line".into(), point)])
    )
    .is_err());
    let mut large = Objects::new();
    for index in 0..128 {
        let id = format!("text-{index}");
        large.insert(
            id.clone(),
            json!({"id":id,"type":"text","zIndex":0,
        "style":{"color":"#000","width":1},"x":0,"y":0,"text":"x".repeat(4096)}),
        );
    }
    assert!(View::from_basis(&basis(), before.size(), large).is_err());
    let nested = (0..12).fold(json!(0), |value, _| json!({"nested":value}));
    assert_eq!(
        before.edited(Objects::from([("shape-a".into(), nested)])),
        Err(Error::Annotation)
    );
}

#[test]
fn full_replacement_has_512_local_changes_but_each_wire_request_still_has_32_limit() {
    let build = |prefix: &str| {
        (0..256)
            .map(|index| {
                let id = format!("{prefix}-{index:03}");
                (id.clone(), rect(&id))
            })
            .collect()
    };
    let before =
        View::from_basis(&basis(), Size::new(120.0, 100.0).unwrap(), build("old")).unwrap();
    let current = View::from_basis(&basis(), before.size(), build("new")).unwrap();
    let changes = before.changes_to(&current).unwrap();
    assert_eq!(changes.len(), 512);
    assert_eq!(changes.iter().filter(|c| c.value.is_null()).count(), 256);
    let request = |changes| {
        Request::prepare(Operation::Apply {
            projection_id: PROJECTION.into(),
            session_id: SESSION.into(),
            op_id: "op-batch".into(),
            base_revision: 1,
            mode_revision: 1,
            changes,
        })
    };
    assert!(request(changes.clone()).is_err());
    for batch in changes.chunks(32) {
        assert!(request(batch.to_vec()).is_ok());
    }
}
