//! 实际系统 ICU 和 DirectWrite 的规范化文字，不用 ASCII 分词替身。
use super::*;
#[test]
fn word_boundaries_preserve_unicode_and_dictionary_segments() {
    for (text, position, expected) in [
        ("Alpha one", 1, "Alpha "),
        ("can't stop", 2, "can't "),
        ("foo_bar baz", 3, "foo_bar "),
        ("12.34 next", 2, "12.34 "),
        ("cafe\u{301} noir", 2, "cafe\u{301} "),
        ("你好世界 测试", 0, "你好"),
        ("你好世界 测试", 2, "世界 "),
        ("ภาษาไทยทดสอบ", 1, "ภาษา"),
        ("A 👩‍👩‍👧‍👦 B", 2, "👩‍👩‍👧‍👦 "),
        ("مرحبا بالعالم", 1, "مرحبا "),
    ] {
        let value = layout(text, Whitespace::Normal, 400.);
        let boundaries = value.word_boundaries().unwrap();
        let range = value.word_range(position, &boundaries).unwrap();
        assert_eq!(value.selected_text(range).unwrap(), expected, "{text}");
    }
}
#[test]
fn word_ranges_use_normalized_space_and_do_not_eat_newlines() {
    let value = layout("Alpha   beta", Whitespace::Normal, 400.);
    let boundaries = value.word_boundaries().unwrap();
    assert_eq!(value.word_range(1, &boundaries).unwrap(), 0..6);
    let pre = layout("Alpha\r\n  beta", Whitespace::PreWrap, 400.);
    let boundaries = pre.word_boundaries().unwrap();
    assert_eq!(
        pre.selected_text(pre.word_range(1, &boundaries).unwrap())
            .unwrap(),
        "Alpha"
    );
    assert!(pre.word_range(pre.text_len() + 1, &boundaries).is_err());
    assert!(pre.word_range(1, &[]).is_err());
}
#[test]
fn word_ranges_reject_surrogate_interior_and_accept_empty_text() {
    let value = layout("A😀 B", Whitespace::Normal, 400.);
    let boundaries = value.word_boundaries().unwrap();
    assert!(value.word_range(2, &boundaries).is_err());
    assert_eq!(value.word_range(1, &boundaries).unwrap(), 1..4);
    let empty = layout("", Whitespace::Normal, 400.);
    assert_eq!(empty.word_boundaries().unwrap(), vec![0]);
    assert_eq!(empty.word_range(0, &[0]).unwrap(), 0..0);
}
