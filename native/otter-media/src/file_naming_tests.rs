use super::*;
fn moment() -> Moment<'static> {
    Moment {
        date: "20261018",
        time: "03040598",
        timestamp: "123456",
    }
}
#[test]
fn defaults_dimensions_and_all_main_placeholders_are_preserved() {
    assert_eq!(
        render(
            "a{shortId}b",
            &Context {
                unit_id: "   ",
                ..Default::default()
            },
            &moment(),
            80
        )
        .unwrap(),
        "ab"
    );
    let context = Context {
        unit_id: "unit-你好世界",
        width: Some(320),
        height: Some(240),
        ..Default::default()
    };
    assert_eq!(
        render(DEFAULT_SAVE_PATTERN, &context, &moment(), 80).unwrap(),
        "Otter_20261018_03040598_320x240"
    );
    assert_eq!(
        render(
            "{app}_{kind}_{label}_{shortId}_{timestamp}",
            &context,
            &moment(),
            80
        )
        .unwrap(),
        "Otter_image_image_你好世界_123456"
    );
    let context = Context {
        app: "X",
        kind: "art",
        label: "标签",
        title: "abcdef",
        process: "p",
        unit_id: "u",
        short_id: "s",
        width: None,
        height: None,
    };
    assert_eq!(
        render(
            "{app}_{kind}_{label}_{title}_{process}_{unitId}_{shortId}_{width}_{height}",
            &context,
            &moment(),
            3
        )
        .unwrap(),
        "X_art_标签_abc_p_u_s__"
    );
}
#[test]
fn malformed_templates_and_resource_limits_fail_explicitly() {
    for pattern in ["", " ", "{", "}", "{}", "{unknown}", "{{title}}", "x{title"] {
        assert_eq!(validate(pattern), Err(Error::Pattern));
    }
    assert_eq!(validate(&"x".repeat(4097)), Err(Error::Capacity));
    assert_eq!(
        render("x", &Context::default(), &moment(), 0),
        Err(Error::TitleLimit)
    );
    let long = "x".repeat(65537);
    assert_eq!(
        render(
            "{label}",
            &Context {
                label: &long,
                ..Default::default()
            },
            &moment(),
            80
        ),
        Err(Error::Capacity)
    );
}
#[test]
fn windows_stem_cleaning_matches_main_and_never_becomes_a_path() {
    for value in ["CONIN$", "CONOUT$", "COM¹", "LPT²", "COM0"] {
        assert_eq!(
            render(
                "{label}",
                &Context {
                    label: value,
                    ..Default::default()
                },
                &moment(),
                80
            )
            .unwrap(),
            format!("_{value}")
        );
    }
    for (input, expected) in [
        ("a/:\\*b... ", "a_b"),
        ("..", "Otter"),
        ("CON", "_CON"),
        ("com9.txt", "_com9.txt"),
        ("LPT1", "_LPT1"),
        ("LPT10", "LPT10"),
    ] {
        assert_eq!(
            render(
                "{label}",
                &Context {
                    label: input,
                    ..Default::default()
                },
                &moment(),
                80
            )
            .unwrap(),
            expected
        );
    }
    let value = render(&"界".repeat(500), &Context::default(), &moment(), 80).unwrap();
    assert_eq!(value.chars().count(), 120);
    assert_eq!(
        render(
            "{label}{process}",
            &Context {
                label: "/",
                process: ":",
                ..Default::default()
            },
            &moment(),
            80
        )
        .unwrap(),
        "_"
    );
}
