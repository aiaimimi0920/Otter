//! Streaming budget checks before usvg allocates filter lists/executes loops; not feature admission or total-runtime guarantees.
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
            // Include merge/function/light children to prevent cumulative-budget bypass through nested structures.
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
                    _ => continue, // Separate semantic admission owns lengths, enums, and references; do not mistake them for pure numbers.
                };
                let (count, first) = list(attr.value(), limit, &mut numbers)?;
                // usvg converts octave to f32 before round, while order truncates directly from f64 toward zero.
                // Sharing f32/round would incorrectly reject 9.5 or f64 values near integers.
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
    // Descriptive nodes produce no filter work; the shared nodes_limit still bounds their XML count.
    node.children()
        .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "title" | "desc"))
        .take(MAX_PRIMITIVES + 1)
        .count()
}

fn list(value: &str, limit: usize, total: &mut usize) -> io::Result<(usize, [f64; 2])> {
    let mut count = 0;
    let mut first = [0.0; 2];
    // Do not collect attacker-controlled Vecs; retain only the first two numbers needed for order/octave.
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_svg_filter_budget_tests.rs"]
mod tests;
