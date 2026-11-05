//! 预算边界测试不代表滤镜准入；真正 worker 仍整体拒绝未开放滤镜。
use super::*;

#[test]
fn convolution_order_matches_actual_usvg_f64_truncation_not_octave_rounding() {
    for (order, columns, rows) in [
        ("9.5", 9, 9),
        ("9.999999999", 9, 9),
        ("3.9 2.9", 3, 2),
        ("1.999999999 1", 1, 1),
    ] {
        let kernel = "1 ".repeat(columns * rows);
        let xml = format!("<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><defs><filter id='f'><feConvolveMatrix order='{order}' kernelMatrix='{kernel}'/></filter></defs><rect width='8' height='8' filter='url(#f)'/></svg>");
        let tree = resvg::usvg::Tree::from_str(&xml, &resvg::usvg::Options::default()).unwrap();
        let filters = tree.filters();
        assert_eq!(filters.len(), 1);
        let resvg::usvg::filter::Kind::ConvolveMatrix(matrix) = filters[0].primitives()[0].kind()
        else {
            panic!("fixture did not produce a convolution: {order}");
        };
        assert_eq!(matrix.matrix().columns(), columns as u32);
        assert_eq!(matrix.matrix().rows(), rows as u32);
        let doc = super::super::policy::parse(&xml).unwrap();
        assert!(
            validate(&doc).is_ok(),
            "bounded renderer order rejected: {order}"
        );
    }
    for order in ["0.9", "10", "10 1", "1 10"] {
        assert!(
            !accepts(&format!("<feConvolveMatrix order='{order}'/>")),
            "{order}"
        );
    }
    assert!(accepts("<feTurbulence numOctaves='9.49'/>"));
    assert!(!accepts("<feTurbulence numOctaves='9.5'/>"));
}

fn accepts(content: &str) -> bool {
    let xml = format!("<svg xmlns='http://www.w3.org/2000/svg'>{content}</svg>");
    let doc = super::super::policy::parse(&xml).unwrap();
    validate(&doc).is_ok()
}

#[test]
fn finite_defaults_and_bounded_lists_do_not_change_the_document() {
    for content in [
        "<path d='M0 0H1'/>",
        "<filter><feTurbulence/></filter>",
        "<filter><feTurbulence numOctaves='0'/></filter>",
        "<filter><feTurbulence numOctaves='9'/></filter>",
        "<filter><feConvolveMatrix order='3, 2' kernelMatrix='0 1 0, 0 1 0'/></filter>",
        "<filter><feGaussianBlur stdDeviation='2.5 0'/></filter>",
        "<filter><feComponentTransfer><feFuncR tableValues=''/></feComponentTransfer></filter>",
    ] {
        assert!(accepts(content), "{content}");
    }
    let xml = "<svg xmlns='http://www.w3.org/2000/svg'><filter><feTurbulence numOctaves='9'/></filter></svg>";
    let doc = super::super::policy::parse(xml).unwrap();
    validate(&doc).unwrap();
    assert_eq!(doc.input_text(), xml);
    assert!(super::super::policy::validate(&doc).is_err());
}

#[test]
fn numeric_overflows_large_loops_and_invalid_lists_fail_before_renderer() {
    for value in [
        "10",
        "4294967295",
        "1e39",
        "1e999",
        "NaN",
        "Infinity",
        "-1",
        "1 2",
        "",
    ] {
        assert!(
            !accepts(&format!(
                "<filter><feTurbulence numOctaves='{value}'/></filter>"
            )),
            "{value}"
        );
    }
    for value in ["10", "1 10", "0", "-1", "1e39", "1 2 3", ""] {
        assert!(
            !accepts(&format!(
                "<filter><feConvolveMatrix order='{value}'/></filter>"
            )),
            "{value}"
        );
    }
    for (name, attr) in [
        ("feTurbulence", "baseFrequency"),
        ("feMorphology", "radius"),
        ("feGaussianBlur", "stdDeviation"),
        ("feFuncR", "tableValues"),
        ("feComposite", "k1"),
    ] {
        for value in ["1e39", "1e999", "NaN", "Infinity", "1 garbage"] {
            assert!(
                !accepts(&format!("<{name} {attr}='{value}'/>")),
                "{name}/{attr}/{value}"
            );
        }
    }
}

#[test]
fn definition_and_per_filter_limits_include_unused_content() {
    assert!(accepts(&"<filter/>".repeat(MAX_FILTERS)));
    assert!(!accepts(&"<filter/>".repeat(MAX_FILTERS + 1)));
    let primitives = "<feFlood/>".repeat(MAX_PRIMITIVES);
    assert!(accepts(&format!("<filter>{primitives}</filter>")));
    assert!(!accepts(&format!(
        "<filter>{primitives}<feFlood/></filter>"
    )));
    let group = format!("<filter>{primitives}</filter>");
    assert!(accepts(
        &group.repeat(MAX_TOTAL_PRIMITIVES / MAX_PRIMITIVES)
    ));
    assert!(!accepts(&format!(
        "{}<filter><feFlood/></filter>",
        group.repeat(4)
    )));
}

#[test]
fn nested_merge_functions_and_lights_cannot_hide_unbounded_work() {
    let nodes = "<feMergeNode/>".repeat(MAX_PRIMITIVES);
    assert!(accepts(&format!(
        "<filter><feMerge>{nodes}</feMerge></filter>"
    )));
    assert!(!accepts(&format!(
        "<filter><feMerge>{nodes}<feMergeNode/></feMerge></filter>"
    )));
    for (parent, child, limit) in [
        ("feComponentTransfer", "feFuncR", 4),
        ("feDiffuseLighting", "fePointLight", 1),
        ("feSpecularLighting", "feSpotLight", 1),
    ] {
        let child = format!("<{child}/>");
        assert!(accepts(&format!(
            "<filter><{parent}>{}</{parent}></filter>",
            child.repeat(limit)
        )));
        assert!(!accepts(&format!(
            "<filter><{parent}>{}</{parent}></filter>",
            child.repeat(limit + 1)
        )));
    }
}

#[test]
fn list_and_document_totals_are_independently_bounded() {
    let table = "0 ".repeat(MAX_TABLE);
    assert!(accepts(&format!("<feFuncR tableValues='{table}'/>")));
    assert!(!accepts(&format!("<feFuncR tableValues='{table}0'/>")));
    let item = format!("<feFuncR tableValues='{table}'/>");
    assert!(accepts(&item.repeat(MAX_NUMBERS / MAX_TABLE)));
    assert!(!accepts(&format!(
        "{}<feFuncR tableValues='0'/>",
        item.repeat(MAX_NUMBERS / MAX_TABLE)
    )));
    assert!(accepts(&format!(
        "<feConvolveMatrix order='9' kernelMatrix='{}'/>",
        "1 ".repeat(81)
    )));
    assert!(!accepts(&format!(
        "<feConvolveMatrix order='9' kernelMatrix='{}'/>",
        "1 ".repeat(82)
    )));
}
