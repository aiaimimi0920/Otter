use super::*;
use serde_json::json;

fn declaration(id: &str) -> Value {
    json!({"id":id,"label":"  视图🙂  ","fullSize":{"width":640,"height":480}})
}

#[test]
fn normalization_keeps_first_valid_view_original_label_and_default_precedence() {
    let mut invalid = declaration("bad");
    invalid["fullSize"]["width"] = json!(0);
    let mut first = declaration("main");
    first["fullSize"]["width"] = json!(640.0);
    let mut duplicate = declaration("main");
    duplicate["label"] = json!("must not replace");
    let manifest = json!({"defaultViewId":"details","views":[
        invalid, first, duplicate, declaration("details")
    ]});
    let views = Views::from_manifest(Some(&manifest));
    assert_eq!(views.entries().len(), 2);
    assert_eq!(views.entries()[0].label, "  视图🙂  ");
    assert_eq!(views.resolve(Some("main")).unwrap().id, "main");
    assert_eq!(views.resolve(Some("missing")).unwrap().id, "details");
    let manifest = json!({"defaultViewId":"missing","views":[declaration("first")]});
    let views = Views::from_manifest(Some(&manifest));
    assert_eq!(views.resolve(None).unwrap().id, "first");
}

#[test]
fn malformed_entries_are_filtered_without_reserving_their_ids() {
    let mut values = vec![Value::Null, json!(false), json!({})];
    for id in [
        "",
        "bad view",
        "点",
        "bad\\view",
        "bad\nview",
        &"x".repeat(161),
    ] {
        values.push(declaration(id));
    }
    for label in ["", " \t\n\u{feff}", &"🙂".repeat(41)] {
        let mut value = declaration("main");
        value["label"] = json!(label);
        values.push(value);
    }
    for size in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!(16385),
        json!("640"),
        Value::Null,
    ] {
        for key in ["width", "height"] {
            let mut value = declaration("main");
            value["fullSize"][key] = size.clone();
            values.push(value);
        }
    }
    values.push(declaration("main"));
    let manifest = json!({"views":values});
    let views = Views::from_manifest(Some(&manifest));
    assert_eq!(views.entries().len(), 1);
    assert_eq!(views.entries()[0].id, "main");
}

#[test]
fn utf16_ecmascript_whitespace_and_exact_dimension_boundaries_match_baseline() {
    let mut valid = declaration("vendor/example.v1:wide-mode");
    valid["label"] = json!("🙂".repeat(40));
    valid["fullSize"] = json!({"width":1.0,"height":16384.0});
    let mut next = declaration("x".repeat(160).as_str());
    next["label"] = json!("\u{0085}");
    let manifest = json!({"views":[valid,next]});
    let views = Views::from_manifest(Some(&manifest));
    assert_eq!(views.entries().len(), 2);
    assert_eq!(
        (views.entries()[0].width, views.entries()[0].height),
        (1, 16384)
    );
    assert_eq!(views.entries()[1].label, "\u{0085}");
    for manifest in [None, Some(Value::Null), Some(json!({"views":{}}))] {
        let views = Views::from_manifest(manifest.as_ref());
        assert!(views.entries().is_empty());
        assert!(views.resolve(None).is_none());
    }
}

#[test]
fn capability_projects_only_its_own_surface_metadata_and_id_wire_cannot_bypass_validation() {
    let capability = json!({"id":"art","surface":{"views":[declaration("wrong")]},
        "metadata":{"capabilities":{"surface":{"views":[declaration("real")]}}}});
    let capability = crate::loom_capability::Capability::new(&capability).unwrap();
    assert_eq!(capability.surface_views().entries()[0].id, "real");
    let capability = json!({"id":"empty"});
    assert!(crate::loom_capability::Capability::new(&capability)
        .unwrap()
        .surface_views()
        .entries()
        .is_empty());
    for id in ["main", "a/b:c.d_e-f", &"x".repeat(160)] {
        let value = Id::parse(id).unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(serde_json::from_slice::<Id>(&bytes).unwrap(), value);
        assert_eq!(value.as_str(), id);
    }
    for value in [
        json!(""),
        json!("bad space"),
        json!("x".repeat(161)),
        json!(null),
        json!(1),
    ] {
        assert!(serde_json::from_value::<Id>(value).is_err());
    }
}
