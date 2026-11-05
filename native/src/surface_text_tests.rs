use super::*;
#[test]
fn zero_layout_extent_can_be_measured_without_faking_a_positive_width() {
    let engine = Engine::new().unwrap();
    for size in [[0., 100.], [100., 0.], [0., 0.]] {
        let layout = engine
            .layout("A B", style(Whitespace::Normal), size[0], size[1])
            .unwrap();
        assert!(layout
            .size()
            .iter()
            .all(|value| value.is_finite() && *value >= 0.));
        assert!(layout.baseline().is_finite());
    }
    assert!(engine
        .layout("A", style(Whitespace::Normal), -1., 100.)
        .is_err());
}
#[test]
fn fractional_positive_layout_and_font_sizes_are_not_rounded_to_zero() {
    let mut value = style(Whitespace::Normal);
    value.size = 0.5;
    let layout = Engine::new().unwrap().layout("a", value, 0.5, 0.5).unwrap();
    assert!(layout.size()[0] > 0.);
}
#[test]
fn explicit_line_height_uses_uniform_spacing_and_actual_font_baseline() {
    let engine = Engine::new().unwrap();
    let natural = engine
        .layout("first\nsecond", style(Whitespace::Pre), 400., 1000.)
        .unwrap();
    let mut styled = style(Whitespace::Pre);
    styled.line_height = Some(40.);
    let spaced = engine.layout("first\nsecond", styled, 400., 1000.).unwrap();
    assert_eq!(spaced.line_count(), 2);
    assert!((spaced.size()[1] - 80.).abs() < 0.01);
    assert!(spaced.baseline() > natural.baseline());
    let mut invalid = style(Whitespace::Pre);
    invalid.line_height = Some(f32::NAN);
    assert!(engine.layout("text", invalid, 400., 1000.).is_err());
}
fn style(whitespace: Whitespace) -> Style<'static> {
    Style {
        family: "Segoe UI",
        size: 16.,
        weight: 400,
        whitespace,
        align: Align::Start,
        line_height: None,
    }
}
#[test]
fn directwrite_measures_actual_wrapping_without_estimating_character_width() {
    let engine = Engine::new().unwrap();
    let text = "原生文字测量 native text wrapping native text wrapping";
    let narrow = engine
        .layout(text, style(Whitespace::Normal), 70., 1000.)
        .unwrap();
    let wide = engine
        .layout(text, style(Whitespace::Normal), 1000., 1000.)
        .unwrap();
    let nowrap = engine
        .layout(text, style(Whitespace::Nowrap), 70., 1000.)
        .unwrap();
    assert!(narrow.line_count() > wide.line_count());
    assert_eq!(nowrap.line_count(), 1);
    assert!(narrow.size()[1] > wide.size()[1]);
    assert!(nowrap.size()[0] > 70.);
    let pre = engine
        .layout("first\r\nsecond", style(Whitespace::Pre), 1000., 1000.)
        .unwrap();
    assert_eq!(pre.line_count(), 2);
}
#[test]
fn whitespace_preserves_unicode_and_rejects_unbounded_or_invalid_input() {
    let decode = |text: &str, mode| String::from_utf16(&content(text, mode).unwrap()).unwrap();
    assert_eq!(
        decode("a \t\r\nb\u{a0}c😀", Whitespace::Normal),
        "a b\u{a0}c😀"
    );
    assert_eq!(decode("a \t\r\nb", Whitespace::PreWrap), "a \t\nb");
    assert!(content(&"x".repeat(65537), Whitespace::Normal).is_err());
    assert!(content("a\0b", Whitespace::Normal).is_err());
    let engine = Engine::new().unwrap();
    assert!(engine
        .layout("text", style(Whitespace::Normal), f32::NAN, 100.)
        .is_err());
    let mut bad = style(Whitespace::Normal);
    bad.family = "bad\0font";
    assert!(engine.layout("text", bad, 100., 100.).is_err());
}
