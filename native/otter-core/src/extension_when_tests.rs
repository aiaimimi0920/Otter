//! 对照主干 when 的 grammar、未知属性 fail-closed、严格类型与预算。
use super::*;
fn context(kind: Option<UnitKind>, has_image: bool) -> Context<'static> {
    Context {
        kind,
        has_image,
        attachment_types: &["text.ocr", "quote'\\value"],
    }
}
#[test]
fn supported_grammar_matches_main_context_and_operator_precedence() {
    let source = r#"unit.kind == "sticker" && unit.hasImage && attachment.has("text.ocr")"#;
    let predicate = Predicate::compile(Some(source)).unwrap();
    assert!(predicate.evaluate(&context(Some(UnitKind::Sticker), true)));
    assert!(!predicate.evaluate(&context(Some(UnitKind::Art), true)));
    assert!(!predicate.evaluate(&context(Some(UnitKind::Sticker), false)));
    for (source, expected) in [
        ("!unit.hasImage || unit.kind != 'art'", true),
        ("false && true || true", true),
        ("false && (true || true)", false),
        ("true == false", false),
        ("unit.hasImage != false", true),
        ("!false == false", false),
        ("attachment.has('absent')", false),
        (r"attachment.has('quote\'\\value')", true),
    ] {
        assert_eq!(
            Predicate::compile(Some(source))
                .unwrap()
                .evaluate(&context(Some(UnitKind::Sticker), true)),
            expected,
            "{source}"
        );
    }
    assert!(!Predicate::compile(Some("unit.kind == 'sticker'"))
        .unwrap()
        .evaluate(&context(None, true)));
}
#[test]
fn unknown_properties_hide_the_whole_predicate_even_under_negation_or_type_comparison() {
    for source in [
        "future.context.key",
        "!future.context.key",
        "future.context.key != 'sticker'",
        "unit.hasImage || !future.context.key",
        "'text' && future.key",
    ] {
        assert!(
            !Predicate::compile(Some(source))
                .unwrap()
                .evaluate(&context(Some(UnitKind::Sticker), true)),
            "{source}"
        );
    }
}
#[test]
fn invalid_types_syntax_code_and_unsupported_escapes_are_rejected() {
    for source in [
        "'sticker'",
        "unit.kind",
        "unit.kind && unit.hasImage",
        "!unit.kind",
        "unit.kind == true",
        "unit.kind &&&& unit.hasImage",
        "globalThis.alert('unsafe')",
        "attachment.has('')",
        "attachment.has(true)",
        "attachment.has('a', 'b')",
        "true true",
        "true === true",
        "1",
        "'unterminated",
        r"attachment.has('\n')",
    ] {
        assert!(Predicate::compile(Some(source)).is_err(), "{source}");
    }
}
#[test]
fn default_unicode_whitespace_and_utf16_limits_match_main() {
    for source in [None, Some(""), Some(" \t\r\n\u{feff}\u{a0}")] {
        assert!(Predicate::compile(source)
            .unwrap()
            .evaluate(&context(None, false)));
    }
    assert!(Predicate::compile(Some("\u{85}true")).is_err());
    let at_limit = format!("attachment.has('{}')", "😀".repeat(1015));
    assert_eq!(at_limit.encode_utf16().count(), 2048);
    assert!(Predicate::compile(Some(&at_limit)).is_ok());
    assert!(matches!(
        Predicate::compile(Some(&at_limit.replace("')", "😀')"))),
        Err(Error::Budget)
    ));
    assert!(matches!(
        Predicate::compile(Some(&format!("{at_limit}!"))),
        Err(Error::Budget)
    ));
}
#[test]
fn expression_tokens_nodes_and_recursion_remain_bounded() {
    for source in [
        format!("{}true{}", "(".repeat(20), ")".repeat(20)),
        format!("{}true", "!".repeat(256)),
        ["true"; 65].join(" || "),
        ["true"; 129].join(" || "),
    ] {
        assert!(
            matches!(Predicate::compile(Some(&source)), Err(Error::Budget)),
            "{source}"
        );
    }
    assert!(Predicate::compile(Some(&["true"; 64].join(" || "))).is_ok());
}
#[test]
fn quoted_operator_values_follow_the_original_parser_contract() {
    assert!(!Predicate::compile(Some("'!' true"))
        .unwrap()
        .evaluate(&context(None, false)));
    assert!(Predicate::compile(Some("'attachment.has'('text.ocr')"))
        .unwrap()
        .evaluate(&context(None, false)));
}
