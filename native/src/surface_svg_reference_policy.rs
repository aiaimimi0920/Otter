//! 结构引用图准入；拒绝环及过深展开，不依赖 renderer 栈溢出。滤镜功能准入仍独立。
use super::invalid;
use resvg::usvg::roxmltree::Document;
use std::{collections::HashMap, io};
const MAX_DEPTH: usize = 128;
pub(super) fn validate(doc: &Document<'_>) -> io::Result<()> {
    if !doc.descendants().any(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "mask" | "symbol" | "use" | "pattern" | "marker" | "textPath" | "filter"
            )
    }) {
        return Ok(());
    }
    let mut ids = HashMap::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        if let Some(id) = node.attribute("id") {
            // 不依赖 renderer 对重复 ID 的 first/last-wins 选择，否则引用图可能不同。
            if ids.insert(id, node.id().get() as usize).is_some() {
                return Err(invalid());
            }
        }
    }
    let mut edges = vec![Vec::new(); doc.descendants().count()];
    // marker 属性可继承。按文档先序传递状态，避免逐节点回溯深祖先造成平方开销。
    let mut inherited_markers = vec![[None::<usize>; 3]; edges.len()];
    for node in doc.descendants().filter(|n| n.is_element()) {
        let links = &mut edges[node.id().get() as usize];
        links.extend(
            node.children()
                .filter(|n| n.is_element())
                .map(|n| n.id().get() as usize),
        );
        // mask 定义自身的 mask 属性按浏览器行为在 normalize 移除，不形成绘制依赖。
        for attribute in ["mask", "clip-path", "filter"] {
            if attribute == "mask" && node.tag_name().name() == "mask" {
                continue;
            }
            if let Some(value) = node.attribute(attribute).filter(|v| *v != "none") {
                let iri = svgtypes::FuncIRI::from_str(value).map_err(|_| invalid())?;
                if let Some(target) = ids.get(iri.0) {
                    links.push(*target);
                }
            }
        }
        let mut marker_refs = node
            .parent()
            .map(|parent| inherited_markers[parent.id().get() as usize])
            .unwrap_or([None; 3]);
        for (index, attribute) in ["marker-start", "marker-mid", "marker-end"]
            .iter()
            .enumerate()
        {
            if let Some(value) = node.attribute(*attribute) {
                marker_refs[index] = if value == "none" {
                    None
                } else {
                    let iri = svgtypes::FuncIRI::from_str(value).map_err(|_| invalid())?;
                    ids.get(iri.0).copied()
                };
            }
        }
        links.extend(marker_refs.iter().flatten().copied());
        inherited_markers[node.id().get() as usize] = marker_refs;
        // 图案内容可通过 fill/stroke 重新引用图案；须在 renderer 前拒绝递归。
        for attribute in ["fill", "stroke"] {
            if let Some(value) = node.attribute(attribute) {
                if let svgtypes::Paint::FuncIRI(id, _) =
                    svgtypes::Paint::from_str(value).map_err(|_| invalid())?
                {
                    if let Some(target) = ids.get(id) {
                        links.push(*target);
                    }
                }
            }
        }
        if matches!(
            node.tag_name().name(),
            "use" | "pattern" | "linearGradient" | "radialGradient" | "textPath" | "filter"
        ) {
            // 两种 href 均保守计入，避免解析器优先级差异绕过准入。
            for attr in node.attributes().filter(|a| a.name() == "href") {
                if let Some(target) = attr.value().strip_prefix('#').and_then(|id| ids.get(id)) {
                    links.push(*target);
                }
            }
        }
    }
    check(&edges)
}
fn check(edges: &[Vec<usize>]) -> io::Result<()> {
    let mut state = vec![0u8; edges.len()];
    let mut depths = vec![0usize; edges.len()];
    let mut stack = Vec::new();
    for root in 0..edges.len() {
        if state[root] == 2 {
            continue;
        }
        stack.push((root, false));
        while let Some((node, exiting)) = stack.pop() {
            if exiting {
                depths[node] = 1 + edges[node].iter().map(|n| depths[*n]).max().unwrap_or(0);
                if depths[node] > MAX_DEPTH {
                    return Err(invalid());
                }
                state[node] = 2;
            } else {
                match state[node] {
                    1 => return Err(invalid()),
                    2 => continue,
                    _ => {}
                }
                state[node] = 1;
                stack.push((node, true));
                stack.extend(edges[node].iter().map(|n| (*n, false)));
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_svg_filter_templates_reuse_the_bounded_reference_graph() {
        for source in [
            "<svg><filter id='f' href='#f'/></svg>",
            "<svg><filter id='a' href='#b'/><filter id='b' href='#a'/></svg>",
            "<svg><filter id='f'/><path id='f'/></svg>",
        ] {
            assert!(
                validate(&Document::parse(source).unwrap()).is_err(),
                "{source}"
            );
        }
        let valid = "<svg><defs><filter id='base'><feFlood/></filter><filter id='f' href='#base'/></defs><g filter='url(#f)'><rect/></g></svg>";
        assert!(validate(&Document::parse(valid).unwrap()).is_ok());
        let mut chain = String::from("<svg>");
        for index in 0..MAX_DEPTH {
            chain.push_str(&format!("<filter id='f{index}' href='#f{}'/>", index + 1));
        }
        chain.push_str(&format!("<filter id='f{MAX_DEPTH}'/></svg>"));
        assert!(validate(&Document::parse(&chain).unwrap()).is_err());
    }
    #[test]
    fn surface_svg_marker_graph_rejects_recursive_content_before_renderer() {
        assert!(validate(&Document::parse("<svg marker-end='url(#m)'><defs><marker id='m' marker-end='none'><path/></marker></defs><path/></svg>").unwrap()).is_ok());
        for source in [
            "<svg><marker id='m'><path marker-end='url(#m)'/></marker></svg>",
            "<svg><marker id='m'><use href='#p'/></marker><path id='p' marker-start='url(#m)'/></svg>",
            "<svg><marker id='a'><path marker-mid='url(#b)'/></marker><marker id='b'><path marker-end='url(#a)'/></marker></svg>",
            "<svg marker-end='url(#m)'><defs><marker id='m'><path/></marker></defs><path/></svg>",
        ] {
            assert!(validate(&Document::parse(source).unwrap()).is_err(), "{source}");
        }
        assert!(validate(
            &Document::parse(
                "<svg><marker id='m'><rect/></marker><path marker-end='url(#m)'/></svg>"
            )
            .unwrap()
        )
        .is_ok());
    }
    #[test]
    fn surface_svg_pattern_graph_rejects_paint_template_and_use_cycles() {
        for source in [
            "<svg><pattern id='p' href='#q'/><pattern id='q' href='#p'/></svg>",
            "<svg><pattern id='p'><rect fill='url(#p)'/></pattern></svg>",
            "<svg><pattern id='p'><use href='#r'/></pattern><rect id='r' stroke='url(#p)'/></svg>",
            "<svg><pattern id='p'/><pattern id='p'/></svg>",
        ] {
            assert!(
                validate(&Document::parse(source).unwrap()).is_err(),
                "{source}"
            );
        }
        let doc = Document::parse("<svg><pattern id='p' href='#base'/><pattern id='base'><rect fill='red'/></pattern><rect fill='url(#p)'/></svg>").unwrap();
        assert!(validate(&doc).is_ok());
    }
    #[test]
    fn surface_svg_mask_graph_resolves_use_and_rejects_ambiguous_ids() {
        for source in [
            "<svg><mask id='m'><use href='#p'/></mask><g id='p' mask='url(#m)'/></svg>",
            "<svg><mask id='m'/><g id='m'/></svg>",
            "<svg><mask id='m'><use href='#p'/></mask><g id='p'><use href='#q'/></g><g id='q' mask='url(#m)'/></svg>",
        ] {
            let doc=Document::parse(source).unwrap();
            assert!(validate(&doc).is_err(), "{source}");
        }
        let doc = Document::parse(
            "<svg><mask id='m' mask='url(#m)'><rect/></mask><rect mask='url(#m)'/></svg>",
        )
        .unwrap();
        assert!(validate(&doc).is_ok());
    }
    #[test]
    fn surface_svg_mask_graph_rejects_long_cycles_and_depth_without_recursion() {
        assert!(check(&[vec![1], vec![2], vec![0]]).is_err());
        assert!(check(&[vec![1, 2], vec![2], vec![]]).is_ok());
        let chain: Vec<_> = (0..MAX_DEPTH + 1)
            .map(|i| if i < MAX_DEPTH { vec![i + 1] } else { vec![] })
            .collect();
        assert!(check(&chain).is_err());
        assert!(check(
            &chain[1..]
                .iter()
                .map(|v| v.iter().map(|n| n - 1).collect())
                .collect::<Vec<_>>()
        )
        .is_ok());
    }
}
