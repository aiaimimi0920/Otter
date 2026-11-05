use super::*;
fn context() -> Context {
    Context {
        axis: 200.,
        viewport: [1000., 800.],
        container: [400., 300.],
        root_font: 16.,
        font: 20.,
        fraction: Some(50.),
    }
}
#[test]
fn lengths_resolve_without_css_or_script_execution() {
    for (text, expected) in [
        ("20px", 20.),
        ("25%", 50.),
        ("2rem", 32.),
        ("2em", 40.),
        ("10vw", 100.),
        ("10vh", 80.),
        ("10cqw", 40.),
        ("10cqh", 30.),
        ("2fr", 100.),
        ("min(10cqw, 10cqh)", 30.),
        ("min(90%, calc(100% - min(50%, 80px)))", 120.),
        ("min(calc(20% + 10px), calc(100% - min(50%, 80px)))", 50.),
    ] {
        assert_eq!(
            Length::parse(&Value::from(text))
                .unwrap()
                .resolve(context()),
            Some(expected),
            "{text}"
        );
    }
    assert_eq!(
        Length::parse(&Value::from(8192))
            .unwrap()
            .resolve(context()),
        Some(8192.)
    );
    assert_eq!(
        Length::parse(&Value::from("auto"))
            .unwrap()
            .resolve(context()),
        None
    );
}
#[test]
fn invalid_lengths_and_unavailable_layout_context_are_not_guessed() {
    for value in [
        serde_json::json!(-1),
        serde_json::json!(8193),
        serde_json::json!(null),
        serde_json::json!(true),
    ] {
        assert!(Length::parse(&value).is_none());
    }
    for text in [
        "1e3px",
        "-1px",
        ".5px",
        "1.px",
        "1001%",
        "url(file:///secret)",
        "calc(1px + 2px)",
        "min(2px,3px)",
        "min(101%,calc(100%-min(50%,10px)))",
        "1px; color:red",
    ] {
        assert!(Length::parse(&Value::from(text)).is_none(), "{text}");
    }
    let mut c = context();
    c.fraction = None;
    assert_eq!(Length::parse(&Value::from("1fr")).unwrap().resolve(c), None);
    c.axis = f32::NAN;
    assert_eq!(Length::parse(&Value::from("1px")).unwrap().resolve(c), None);
}
