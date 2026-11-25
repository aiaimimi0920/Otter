//! 数值/文本夹具已用本地 Node 的 Number/String 检查；不等于全部 JS 边界均已验证。
use super::*;
fn widget(name: &str) -> Widget<'_> {
    Widget {
        name,
        min: None,
        max: None,
    }
}
fn raw<'a>(value: Output<'a>) -> Option<Value<'a>> {
    Some(Value::Borrowed(value))
}
#[test]
fn finite_numbers_radix_rounding_and_js_whitespace() {
    for (text, expected) in [
        ("", 0.0),
        ("  ", 0.0),
        ("0b11", 3.0),
        ("0o17", 15.0),
        ("0x20000000000001", 9007199254740992.0),
        ("0x20000000000003", 9007199254740996.0),
        ("\u{feff}1\u{feff}", 1.0),
        ("1.25e2", 125.0),
    ] {
        assert_eq!(
            coerce(&widget("number"), Output::Image(text)),
            raw(Output::Number(expected)),
            "{text:?}"
        );
    }
    for text in [
        "+0x1",
        "-0b1",
        "0x",
        "NaN",
        "Infinity",
        "1e999",
        "\u{0085}1\u{0085}",
        "1_000",
    ] {
        assert_eq!(
            coerce(&widget("number"), Output::Image(text)),
            None,
            "{text:?}"
        );
    }
    assert_eq!(
        coerce(&widget("number"), Output::Boolean(true)),
        raw(Output::Number(1.0))
    );
    assert_eq!(
        coerce(&widget("number"), Output::Array(&[Output::Image("2")])),
        raw(Output::Number(2.0))
    );
    assert_eq!(
        coerce(&widget("number"), Output::Array(&[Output::Null])),
        raw(Output::Number(0.0))
    );
}
#[test]
fn numeric_clamps_follow_min_then_max_and_ignore_nonfinite_bounds() {
    let limits = Widget {
        name: "SLIDER",
        min: Some(10.0),
        max: Some(5.0),
    };
    assert_eq!(
        coerce(&limits, Output::Number(1.0)),
        raw(Output::Number(5.0))
    );
    let limits = Widget {
        name: "number",
        min: Some(f64::NAN),
        max: Some(f64::INFINITY),
    };
    assert_eq!(
        coerce(&limits, Output::Number(7.0)),
        raw(Output::Number(7.0))
    );
    assert_eq!(coerce(&limits, Output::Number(f64::INFINITY)), None);
}
#[test]
fn booleans_accept_only_baseline_spellings_and_number_rule() {
    for text in ["true", " YES ", "1", "on"] {
        assert_eq!(
            coerce(&widget("checkbox"), Output::Image(text)),
            raw(Output::Boolean(true))
        );
    }
    for text in ["false", " No ", "0", "off"] {
        assert_eq!(
            coerce(&widget("switch"), Output::Image(text)),
            raw(Output::Boolean(false))
        );
    }
    for text in ["", "2", "truthy"] {
        assert_eq!(coerce(&widget("switch"), Output::Image(text)), None);
    }
    assert_eq!(
        coerce(&widget("switch"), Output::Number(f64::NAN)),
        raw(Output::Boolean(true))
    );
    assert_eq!(
        coerce(&widget("switch"), Output::Number(-0.0)),
        raw(Output::Boolean(false))
    );
}
#[test]
fn number_text_thresholds_and_negative_zero_match_node_fixtures() {
    for (value, expected) in [
        (1e21, "1e+21"),
        (1e20, "100000000000000000000"),
        (1e-7, "1e-7"),
        (1e-6, "0.000001"),
        (-0.0, "0"),
        (1000000000000000100.0, "1000000000000000100"),
    ] {
        assert_eq!(
            coerce(&widget("text"), Output::Number(value)),
            Some(Value::Text(expected.into()))
        );
    }
    assert_eq!(
        coerce(&widget("color"), Output::Boolean(false)),
        Some(Value::Text("false".into()))
    );
    assert_eq!(
        coerce(
            &widget("text"),
            Output::Array(&[
                Output::Image("a"),
                Output::Null,
                Output::Array(&[Output::Number(2.0), Output::Boolean(true)])
            ])
        ),
        Some(Value::Text("a,,2,true".into()))
    );
}
#[test]
fn wrapper_file_unknown_widget_and_text_budget_are_preserved() {
    let fields = [("value", Output::Image("path"))];
    assert_eq!(
        coerce(&widget("file"), Output::Object(&fields)),
        raw(Output::Image("path"))
    );
    assert_eq!(coerce(&widget("image_link"), Output::Image("")), None);
    assert_eq!(coerce(&widget("file"), Output::Number(1.0)), None);
    assert_eq!(
        coerce(&widget("custom"), Output::Object(&fields)),
        raw(Output::Image("path"))
    );
    let oversized = "a".repeat(MAX_TEXT_BYTES + 1);
    assert_eq!(coerce(&widget("text"), Output::Image(&oversized)), None);
    assert_eq!(coerce(&widget("number"), Output::Image(&oversized)), None);
}
