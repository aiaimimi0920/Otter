//! 已通过 policy 的 SVG 到固定 renderer 的语义适配；不读取文件或解析外部引用。
use super::{invalid, viewport::Viewport};
use resvg::usvg::roxmltree::Node;
use std::io;
use svgtypes::Length;

pub(super) fn normalize(
    source: &str,
    root: Node<'_, '_>,
    viewport: &Viewport,
) -> io::Result<String> {
    let mut output = source.to_owned();
    let definitions: std::collections::HashMap<_, _> = root
        .descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| n.attribute("id").map(|id| (id, n)))
        .collect();
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
                // Chrome 的描述子元素阻止模板内容继承；usvg 会先剔除它们，
                // 从而误用 href 内容。空组保留“有自身内容”的语义，不影响属性继承。
                let range = node.range();
                let close = source[range.clone()].rfind("</").ok_or_else(invalid)? + range.start;
                edits.push((
                    close..close,
                    "<g xmlns=\"http://www.w3.org/2000/svg\"/>".into(),
                ));
            }
        }
        if node.tag_name().name() == "symbol" {
            // 走 renderer 的嵌套 SVG 实例分支：use 尺寸只解析一次，内容百分比
            // 使用实例 viewBox。defs 保证未引用的 symbol 仍不直接绘制。
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
        // 浏览器不在 mask 定义元素自身应用 mask；usvg 会应用并递归关联。
        // 只移除此处的 presentation attribute，不影响 mask 子内容上的正常遮罩。
        if node.tag_name().name() == "mask" {
            if let Some(attr) = node
                .attributes()
                .find(|a| a.namespace().is_none() && a.name() == "mask")
            {
                edits.push((attr.range(), String::new()));
            }
        }
        let target = if node.tag_name().name() == "use" {
            // roxmltree attribute("href") 不限定 namespace，不能用于此优先级。
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
        // usvg 的零尺寸嵌套 viewport 会退回父 viewBox 并继续绘制。
        // 保留子树/ID，不删除可能被其他 use 引用的定义。
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
    // 全部 range 来自原文；仅替换独立属性/插入 QName 后，不删除父子元素整段。
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
    Ok(output)
}
fn name_end(source: &str, node: Node<'_, '_>) -> io::Result<usize> {
    let start = node.range().start + 1;
    source[start..]
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .map(|offset| start + offset)
        .ok_or_else(invalid)
}
