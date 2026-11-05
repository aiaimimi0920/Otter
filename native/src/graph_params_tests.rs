use super::*;
#[test]
fn widget_conversion_commits_owned_text_and_rejects_invalid_replacement() {
    let declarations = [
        param("text", None),
        param("number", Some(Output::Number(3.0))),
    ];
    let mut values = Params::new(&declarations, &[]).unwrap();
    let text = Widget {
        name: "text",
        min: None,
        max: None,
    };
    let number = Widget {
        name: "number",
        min: Some(0.0),
        max: Some(10.0),
    };
    assert!(values.apply_widget_linked("text", &text, Some(Output::Number(1e21))));
    assert!(values.apply_widget_linked("number", &number, Some(Output::Image("999"))));
    assert!(!values.apply_widget_linked("number", &number, Some(Output::Image("not-a-number"))));
    assert_eq!(
        values.entries().collect::<Vec<_>>(),
        vec![
            ("text", Some(Output::Image("1e+21"))),
            ("number", Some(Output::Number(10.0)))
        ]
    );
    assert!(values.apply_widget_linked("text", &text, Some(Output::Boolean(false))));
    assert_eq!(
        values.entries().next(),
        Some(("text", Some(Output::Image("false"))))
    );
}
fn param<'a>(id: &'a str, default: Option<Output<'a>>) -> Param<'a> {
    Param {
        id,
        default,
        secret: false,
        widget: Widget::default(),
    }
}
#[test]
fn defaults_manual_null_unknown_and_undefined_preserve_order() {
    let declarations = [
        param("a", Some(Output::Number(1.0))),
        param("b", Some(Output::Boolean(true))),
        param("unset", None),
    ];
    let manual = [("a", Output::Null), ("unknown", Output::Image("manual"))];
    let values = Params::new(&declarations, &manual).unwrap();
    assert_eq!(
        values.entries().collect::<Vec<_>>().as_slice(),
        &[
            ("a", Some(Output::Null)),
            ("b", Some(Output::Boolean(true))),
            ("unset", None),
            ("unknown", Some(Output::Image("manual")))
        ]
    );
}
#[test]
fn secrets_and_controls_are_excluded_from_every_write_path() {
    let declarations = [
        Param {
            id: "token",
            default: Some(Output::Image("secret")),
            secret: true,
            widget: Widget::default(),
        },
        param("safe", None),
        param("__exec_mode", Some(Output::Boolean(true))),
    ];
    let manual = [
        ("token", Output::Image("manual-secret")),
        ("__exec_other", Output::Image("control")),
        ("__ui_resize", Output::Number(2.0)),
        ("force_update", Output::Boolean(true)),
        ("safe", Output::Image("safe")),
    ];
    let mut values = Params::new(&declarations, &manual).unwrap();
    for name in [
        "token",
        "__exec_other",
        "__exec_mode",
        "__ui_resize",
        "force_update",
    ] {
        assert!(!values.apply_linked(name, Some(Output::Image("upstream"))));
    }
    assert_eq!(
        values.entries().collect::<Vec<_>>().as_slice(),
        &[("safe", Some(Output::Image("safe")))]
    );
}
#[test]
fn linked_updates_require_declaration_and_respect_disabled_and_absent_values() {
    let declarations = [
        param("disabled", None),
        param("enabled", Some(Output::Number(3.0))),
    ];
    let manual = [
        ("disabled", Output::Image(DISABLED)),
        ("unknown", Output::Number(4.0)),
    ];
    let mut values = Params::new(&declarations, &manual).unwrap();
    assert!(!values.apply_linked("disabled", Some(Output::Number(5.0))));
    assert!(!values.apply_linked("unknown", Some(Output::Number(5.0))));
    assert!(!values.apply_linked("enabled", None));
    assert!(!values.apply_linked("enabled", Some(Output::Null)));
    assert!(values.apply_linked("enabled", Some(Output::Number(0.0))));
    assert!(values.apply_linked("enabled", Some(Output::Boolean(false))));
    assert_eq!(
        values.entries().collect::<Vec<_>>().as_slice()[1],
        ("enabled", Some(Output::Boolean(false)))
    );
}
#[test]
fn duplicate_secret_declarations_fail_closed_and_special_names_are_data() {
    let declarations = [
        param("key", Some(Output::Image("public"))),
        Param {
            id: "key",
            default: Some(Output::Image("secret")),
            secret: true,
            widget: Widget::default(),
        },
        param("__proto__", Some(Output::Number(1.0))),
    ];
    let mut values = Params::new(&declarations, &[]).unwrap();
    assert!(!values.apply_linked("key", Some(Output::Image("secret"))));
    assert!(values.apply_linked("__proto__", Some(Output::Number(2.0))));
    assert_eq!(
        values.entries().collect::<Vec<_>>().as_slice(),
        &[("__proto__", Some(Output::Number(2.0)))]
    );
    assert!(!internal("__EXEC_mode")); // 主干控制字段匹配区分大小写。
}
#[test]
fn bounded_candidate_rejects_ambiguous_manual_keys() {
    let manual = [("a", Output::Null), ("a", Output::Boolean(true))];
    assert!(matches!(
        Params::new(&[], &manual),
        Err(Error::InvalidOutput)
    ));
    let excess = vec![("a", Output::Null); MAX_PORTS + 1];
    assert!(matches!(Params::new(&[], &excess), Err(Error::Capacity)));
    let malformed = Output::Object(&[
        ("value", Output::Null),
        ("value", Output::Image("ambiguous")),
    ]);
    let declarations = [param("a", Some(Output::Number(1.0)))];
    assert!(matches!(
        Params::new(&declarations, &[("a", malformed)]),
        Err(Error::InvalidOutput)
    ));
    let mut values = Params::new(&declarations, &[]).unwrap();
    assert!(!values.apply_linked("a", Some(malformed)));
    assert_eq!(
        values.entries().collect::<Vec<_>>().as_slice(),
        &[("a", Some(Output::Number(1.0)))]
    );
}
