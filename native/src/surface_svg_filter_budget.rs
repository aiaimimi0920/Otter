//! 在 usvg 分配滤镜列表/执行循环前做流式预算检查；不是功能准入或总运行时保证。
use super::invalid;
use resvg::usvg::roxmltree::{Document, Node};
use std::io;

const MAX_FILTERS: usize = 64;
const MAX_PRIMITIVES: usize = 64;
const MAX_TOTAL_PRIMITIVES: usize = 256;
const MAX_NUMBERS: usize = 16_384;
const MAX_TABLE: usize = 1024;
const MAX_KERNEL_SIDE: f64 = 9.0;
const MAX_OCTAVES: f32 = 9.0;

pub(super) fn validate(doc: &Document<'_>) -> io::Result<()> {
    let mut filters = 0;
    let mut primitives = 0;
    let mut numbers = 0;
    for node in doc.descendants().filter(Node::is_element) {
        let name = node.tag_name().name();
        if name == "filter" {
            filters += 1;
            if filters > MAX_FILTERS || children(node) > MAX_PRIMITIVES {
                return Err(invalid());
            }
        } else if name.starts_with("fe") {
            // 包含 merge/function/light 子节点，防止藏在子结构中绕过累计预算。
            primitives += 1;
            if primitives > MAX_TOTAL_PRIMITIVES {
                return Err(invalid());
            }
            match name {
                "feMerge" if children(node) > MAX_PRIMITIVES => return Err(invalid()),
                "feComponentTransfer" if children(node) > 4 => return Err(invalid()),
                "feDiffuseLighting" | "feSpecularLighting" if children(node) > 1 => {
                    return Err(invalid());
                }
                _ => {}
            }
            for attr in node.attributes() {
                let limit = match attr.name() {
                    "tableValues" => MAX_TABLE,
                    "kernelMatrix" => (MAX_KERNEL_SIDE * MAX_KERNEL_SIDE) as usize,
                    "values" => 20,
                    "order" | "stdDeviation" | "radius" | "baseFrequency" | "kernelUnitLength" => 2,
                    "numOctaves" | "seed" | "divisor" | "bias" | "targetX" | "targetY"
                    | "slope" | "intercept" | "amplitude" | "exponent" | "offset" | "k1" | "k2"
                    | "k3" | "k4" | "scale" | "surfaceScale" | "diffuseConstant"
                    | "specularConstant" | "specularExponent" | "azimuth" | "elevation"
                    | "pointsAtX" | "pointsAtY" | "pointsAtZ" | "limitingConeAngle" => 1,
                    _ => continue, // 长度、枚举与引用由独立语义准入负责，不能误当纯数值。
                };
                let (count, first) = list(attr.value(), limit, &mut numbers)?;
                // usvg 的 octave 先转 f32 再 round；order 则直接从 f64 向零截断。
                // 不能共用 f32/round，否则 9.5 或接近整数的 f64 会被错误拒绝。
                if attr.name() == "numOctaves"
                    && (count != 1 || !(0.0..=MAX_OCTAVES).contains(&(first[0] as f32).round()))
                {
                    return Err(invalid());
                }
                if attr.name() == "order"
                    && (count == 0
                        || first[..count]
                            .iter()
                            .any(|v| !(1.0..=MAX_KERNEL_SIDE).contains(&v.trunc())))
                {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}

fn children(node: Node<'_, '_>) -> usize {
    // 描述节点不产生滤镜工作；其 XML 数量仍由共同 nodes_limit 约束。
    node.children()
        .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "title" | "desc"))
        .take(MAX_PRIMITIVES + 1)
        .count()
}

fn list(value: &str, limit: usize, total: &mut usize) -> io::Result<(usize, [f64; 2])> {
    let mut count = 0;
    let mut first = [0.0; 2];
    // 不收集攻击者控制的 Vec；只保存 order/octave 需要的前两个数。
    for number in svgtypes::NumberListParser::from(value) {
        let number = number.map_err(|_| invalid())?;
        if count == limit || *total == MAX_NUMBERS || !(number as f32).is_finite() {
            return Err(invalid());
        }
        if count < first.len() {
            first[count] = number;
        }
        count += 1;
        *total += 1;
    }
    Ok((count, first))
}

#[cfg(test)]
#[path = "surface_svg_filter_budget_tests.rs"]
mod tests;
