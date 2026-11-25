//! Semantic adaptation from policy-approved SVG to the fixed renderer; no file reads or external-reference resolution.
use super::{invalid, viewport::Viewport};
use resvg::usvg::roxmltree::Node;
use std::io;
use svgtypes::Length;

fn exact_root_dimensions(root: Node<'_, '_>, viewport: &Viewport) -> bool {
    let mut width_ok = false;
    let mut height_ok = false;
    for attr in root.attributes().filter(|a| a.namespace().is_none()) {
        match attr.name() {
            "width" => {
                if attr.value() != viewport.inner_width.to_string() {
                    return false;
                }
                width_ok = true;
            }
            "height" => {
                if attr.value() != viewport.inner_height.to_string() {
                    return false;
                }
                height_ok = true;
            }
            _ => {}
        }
    }
    width_ok && height_ok
}

pub(super) fn normalize_if_needed(
    source: &str,
    root: Node<'_, '_>,
    viewport: &Viewport,
) -> io::Result<Option<String>> {
    let definitions: std::collections::HashMap<_, _> = root
        .descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| n.attribute("id").map(|id| (id, n)))
        .collect();
    let root_attr_edits = root
        .attributes()
        .filter(|a| a.namespace().is_none() && matches!(a.name(), "width" | "height"))
        .count();
    let mut edits: Vec<_> = root
        .attributes()
        .filter(|a| a.namespace().is_none() && matches!(a.name(), "width" | "height"))
        .map(|a| (a.range(), String::new()))
        .collect();
    for node in root.descendants().filter(|n| n.is_element()) {
        if node.tag_name().name() == "pattern" {
            let mut children = node.children().filter(|n| n.is_element()).peekable();
            if children.peek().is_some()
                && children.all(|n| matches!(n.tag_name().name(), "title" | "desc"))
            {
                // Chrome's descriptive children prevent template-content inheritance, but usvg removes them first,
                // incorrectly using href content. Empty groups preserve own-content semantics without affecting attribute inheritance.
                let range = node.range();
                let close = source[range.clone()].rfind("</").ok_or_else(invalid)? + range.start;
                edits.push((
                    close..close,
                    "<g xmlns=\"http://www.w3.org/2000/svg\"/>".into(),
                ));
            }
        }
        if node.tag_name().name() == "symbol" {
            // Use the renderer's nested-SVG instance branch: resolve use dimensions once, and content percentages
            // use the instance viewBox. defs ensures unreferenced symbols still are not painted directly.
            let start = node.range().start;
            let end = name_end(source, node)?;
            let qname = &source[start + 1..end];
            let prefix = qname.strip_suffix("symbol").ok_or_else(invalid)?;
            edits.push((
                start..end,
                format!("<defs xmlns=\"http://www.w3.org/2000/svg\"><{prefix}svg"),
            ));
            let tail = node.range().end;
            if source[..tail].ends_with("/>") {
                edits.push((tail..tail, "</defs>".into()));
            } else {
                let close = source[start..tail].rfind("</").ok_or_else(invalid)? + start;
                edits.push((close..tail, format!("</{prefix}svg></defs>")));
            }
        }
        // Browsers do not apply masks to mask-definition elements themselves; usvg applies them and creates recursive associations.
        // Remove only this presentation attribute here, preserving normal masks on child content.
        if node.tag_name().name() == "mask" {
            if let Some(attr) = node
                .attributes()
                .find(|a| a.namespace().is_none() && a.name() == "mask")
            {
                edits.push((attr.range(), String::new()));
            }
        }
        let target = if node.tag_name().name() == "use" {
            // roxmltree attribute("href") does not restrict namespaces and cannot implement this precedence.
            node.attributes()
                .find(|a| a.namespace().is_none() && a.name() == "href")
                .map(|a| a.value())
                .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
                .and_then(|href| href.strip_prefix('#'))
                .and_then(|id| definitions.get(id).copied())
                .filter(|n| matches!(n.tag_name().name(), "svg" | "symbol"))
        } else {
            None
        };
        if node == root || (node.tag_name().name() != "svg" && target.is_none()) {
            continue;
        }
        // usvg's zero-size nested viewports fall back to the parent's viewBox and continue drawing.
        // Preserve subtrees/IDs rather than deleting definitions potentially referenced by other use elements.
        let zero = ["width", "height"].iter().any(|name| {
            node.attribute(*name)
                .or_else(|| target.and_then(|n| n.attribute(*name)))
                .and_then(|v| v.parse::<Length>().ok())
                .is_some_and(|v| v.number == 0.0)
        });
        if zero {
            if let Some(attr) = node
                .attributes()
                .find(|a| a.namespace().is_none() && a.name() == "display")
            {
                edits.push((attr.range(), "display=\"none\"".into()));
            } else {
                let end = name_end(source, node)?;
                edits.push((end..end, " display=\"none\"".into()));
            }
        }
    }
    if edits.len() == root_attr_edits && exact_root_dimensions(root, viewport) {
        return Ok(None);
    }
    let mut output = source.to_owned();
    // All ranges come from original content; replace only individual attributes/insert after QName without deleting whole parent-child element regions.
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    for (range, value) in edits {
        output.replace_range(range, &value);
    }
    let end = name_end(&output, root)?;
    output.insert_str(
        end,
        &format!(
            " width=\"{}\" height=\"{}\"",
            viewport.inner_width, viewport.inner_height
        ),
    );
    Ok(Some(output))
}
fn name_end(source: &str, node: Node<'_, '_>) -> io::Result<usize> {
    let start = node.range().start + 1;
    source[start..]
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .map(|offset| start + offset)
        .ok_or_else(invalid)
}
