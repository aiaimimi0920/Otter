use super::{catalog::Catalogs, text_for, DisplayLanguage, FluentValue};

#[test]
fn bundled_messages_have_complete_sources_and_valid_arguments() {
    Catalogs::embedded()
        .unwrap()
        .validate_embedded_completeness();
}

#[test]
fn static_labels_are_cached_without_leaking_per_lookup() {
    let first = text_for(DisplayLanguage::English, "common-save");
    let second = text_for(DisplayLanguage::English, "common-save");
    assert_eq!(first, "Save");
    assert_eq!(first.as_ptr(), second.as_ptr());
    assert_eq!(
        text_for(DisplayLanguage::SimplifiedChinese, "common-save"),
        "保存"
    );
}

#[test]
fn missing_translation_falls_back_to_english() {
    let catalogs = Catalogs::new(
        &["label = Source\nmessage = Hello { $name }"],
        &["## Intentionally incomplete test catalog."],
    )
    .unwrap();
    assert_eq!(
        catalogs.text(DisplayLanguage::SimplifiedChinese, "label"),
        "Source"
    );
    assert_eq!(
        catalogs.format(
            DisplayLanguage::SimplifiedChinese,
            "message",
            &[("name", FluentValue::from("Otter"))]
        ),
        "Hello \u{2068}Otter\u{2069}"
    );
}

#[test]
fn translation_can_use_its_own_terms_and_plural_structure() {
    let english = "items = { $count ->\n    [one] One item\n   *[other] { $count } items\n}\n";
    let chinese = "-unit = 项\nitems = { $count } { -unit }\n";
    let catalogs = Catalogs::new(&[english], &[chinese]).unwrap();
    assert!(catalogs
        .format(
            DisplayLanguage::SimplifiedChinese,
            "items",
            &[("count", FluentValue::from(2))]
        )
        .contains('项'));
}

#[test]
fn message_references_contribute_to_the_call_contract() {
    let catalogs = Catalogs::new(
        &["shared = { $name }\nmessage = Hello { shared }"],
        &["message = 你好 { $name }"],
    )
    .unwrap();
    assert!(catalogs
        .format(
            DisplayLanguage::English,
            "message",
            &[("name", FluentValue::from("Otter"))]
        )
        .contains("Otter"));
}

#[test]
fn translation_can_omit_an_english_argument() {
    let catalogs = Catalogs::new(&["message = Hello { $name }"], &["message = 你好"]).unwrap();
    assert_eq!(
        catalogs.format(
            DisplayLanguage::SimplifiedChinese,
            "message",
            &[("name", FluentValue::from("Otter"))]
        ),
        "你好"
    );
}

#[test]
fn invalid_catalog_contracts_are_rejected() {
    for sources in [
        vec!["label = First\nlabel = Duplicate"],
        vec!["label = {"],
        vec!["label = { missing }"],
        vec!["label = { label }"],
        vec!["label = { $count ->\n    [one] Fine\n   *[other] { missing }\n}\n"],
    ] {
        assert!(Catalogs::new(&sources, &[]).is_err());
    }
    assert!(Catalogs::new(&["label = Fine"], &["label = { $unexpected }"]).is_err());
    assert!(Catalogs::new(&["label = Fine"], &["unknown = Extra"]).is_err());
}

#[test]
#[should_panic(expected = "incorrect arguments")]
fn missing_call_argument_is_a_programming_error() {
    Catalogs::new(&["message = { $name }"], &[])
        .unwrap()
        .format(DisplayLanguage::English, "message", &[]);
}

#[test]
#[should_panic(expected = "unknown or parameterized")]
fn parameterized_messages_cannot_be_read_as_static_labels() {
    Catalogs::new(&["message = { $name }"], &[])
        .unwrap()
        .text(DisplayLanguage::English, "message");
}

#[test]
#[should_panic(expected = "duplicate arguments")]
fn duplicate_call_arguments_are_rejected() {
    Catalogs::new(&["message = { $name }"], &[])
        .unwrap()
        .format(
            DisplayLanguage::English,
            "message",
            &[("name", "first".into()), ("name", "second".into())],
        );
}
