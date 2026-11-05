//! Input names must not expose implementation-generated intermediate results.
#[cfg(test)]
mod tests {
    #[test]
    fn surface_svg_reference_fix_does_not_admit_filters() {
        for input in ["result1", "SourceGraphic", "missing"] {
            let source = format!("<svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'><defs><filter id='f'><feOffset in='{input}'/></filter></defs><rect width='4' height='4' filter='url(#f)'/></svg>");
            let error = super::super::decode(source.as_bytes()).err().unwrap();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        }
    }
    fn render(content: &str) -> Vec<u8> {
        let source = format!("<svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'><defs><filter id='f' filterUnits='userSpaceOnUse' x='0' y='0' width='4' height='4'>{content}</filter></defs><rect width='4' height='4' fill='lime' filter='url(#f)'/></svg>");
        let tree = resvg::usvg::Tree::from_str(&source, &Default::default()).unwrap();
        assert_eq!(tree.filters().len(), 1);
        let mut pixels = resvg::tiny_skia::Pixmap::new(4, 4).unwrap();
        resvg::render_checked(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        )
        .unwrap();
        pixels.take()
    }
    #[test]
    fn unnamed_results_are_not_public_aliases() {
        let expected = render("<feFlood flood-color='blue'/>");
        for name in ["result1", "missing"] {
            let actual = render(&format!("<feFlood flood-color='red'/><feFlood flood-color='blue' result='visible'/><feOffset in='{name}'/>"));
            assert_eq!(
                actual, expected,
                "unknown {name} must use previous blue, not an internal red result"
            );
        }
    }
    #[test]
    fn default_and_explicit_inputs_remain_distinct() {
        let blue = render("<feFlood flood-color='blue'/>");
        let red = render("<feFlood flood-color='red'/>");
        assert_eq!(
            render("<feFlood flood-color='red'/><feFlood flood-color='blue'/><feOffset/>"),
            blue
        );
        assert_eq!(render("<feFlood flood-color='red' result='named'/><feFlood flood-color='blue'/><feOffset in='named'/>"), red);
        assert_eq!(render("<feFlood flood-color='red' result='named'/><feFlood flood-color='blue' result='named'/><feOffset in='named'/>"), blue);
    }
    #[test]
    fn forward_names_do_not_expose_an_unnamed_earlier_result() {
        let blue = render("<feFlood flood-color='blue'/>");
        assert_eq!(render("<feFlood flood-color='red'/><feFlood flood-color='blue' result='b'/><feOffset in='result1' result='out'/><feOffset in='out' result='result1'/>"), blue);
    }
    #[test]
    fn merge_and_second_inputs_use_the_same_visibility_rule() {
        let blue = render("<feFlood flood-color='blue'/>");
        let prefix = "<feFlood flood-color='red'/><feFlood flood-color='blue' result='b'/>";
        for tail in [
            "<feMerge><feMergeNode in='result1'/></feMerge>",
            "<feComposite in='SourceGraphic' in2='result1' operator='arithmetic' k3='1'/>",
            "<feBlend in='result1' in2='b' mode='normal'/>",
        ] {
            assert_eq!(render(&format!("{prefix}{tail}")), blue, "{tail}");
        }
    }
    #[test]
    fn generated_name_like_explicit_names_and_first_input_fallback_are_preserved() {
        let red = render("<feFlood flood-color='red'/>");
        assert_eq!(render("<feFlood flood-color='red' result='result2'/><feFlood flood-color='blue'/><feOffset in='result2'/>"), red);
        assert_eq!(
            render("<feOffset in='missing'/>"),
            render("<feOffset in='SourceGraphic'/>")
        );
        assert_eq!(
            render("<feOffset in='SourceAlpha'/>"),
            [0, 0, 0, 255].repeat(16)
        );
    }
}
