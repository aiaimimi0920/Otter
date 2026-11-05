// Copyright 2026 the Otter Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Quotas count executed instances and remain shared across nested render roots.
use crate::{RenderError, tiny_skia, usvg};
const FLOOD: &str =
    "<filter id='f' x='0' y='0' width='100%' height='100%'><feFlood flood-color='red'/></filter>";
const RECT: &str = "<rect width='16' height='16' filter='url(#f)'/>";
fn tree(body: &str) -> usvg::Tree {
    usvg::Tree::from_str(&format!("<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='32' height='16'>{body}</svg>"), &usvg::Options::default()).unwrap()
}
fn run(tree: &usvg::Tree, limit: u64) -> Result<Vec<u8>, RenderError> {
    let mut pixmap = tiny_skia::Pixmap::new(32, 16).unwrap();
    crate::render_with_filter_budget(
        tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
        limit,
    )?;
    Ok(pixmap.take())
}
fn flood_cost() -> u64 {
    let tree = tree(&format!("<defs>{FLOOD}</defs>{RECT}"));
    4096 + crate::filter::work_budget::cost(
        tree.filters()[0].primitives()[0].kind(),
        16,
        16,
        tiny_skia::Transform::identity(),
    )
    .unwrap()
}
#[test]
fn exact_limit_and_normal_pixels() {
    let tree = tree(&format!("<defs>{FLOOD}</defs>{RECT}"));
    let cost = flood_cost();
    assert_eq!(run(&tree, cost - 1), Err(RenderError::FilterBudgetExceeded));
    let normal = run(&tree, cost).unwrap();
    assert_eq!(normal, run(&tree, u64::MAX).unwrap());
    assert!(normal.chunks_exact(4).any(|p| p[3] != 0));
    assert!(run(&self::tree("<rect width='16' height='16' fill='red'/>"), 0).is_ok());
}
#[test]
fn repeated_definition_counts_every_execution_and_resets_per_request() {
    let single = tree(&format!("<defs>{FLOOD}</defs>{RECT}"));
    let repeated = tree(&format!(
        "<defs>{FLOOD}</defs>{RECT}<g transform='translate(16)'>{RECT}</g>"
    ));
    // One XML definition can resolve into multiple usvg filter objects.
    // The quota follows executions, not either representation's definition count.
    let cost = flood_cost();
    assert_eq!(run(&repeated, cost), Err(RenderError::FilterBudgetExceeded));
    assert!(run(&repeated, 2 * cost).is_ok());
    assert!(
        run(&single, cost).is_ok(),
        "a failed request must not poison the next request"
    );
}
#[test]
fn pattern_and_mask_share_the_outer_quota() {
    for body in [
        format!(
            "<defs>{FLOOD}<pattern id='p' patternUnits='userSpaceOnUse' width='16' height='16'>{RECT}</pattern></defs><rect width='16' height='16' fill='url(#p)'/><rect x='16' width='16' height='16' fill='url(#p)'/>"
        ),
        format!(
            "<defs>{FLOOD}<mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='16' height='16'>{RECT}</mask></defs><rect width='16' height='16' mask='url(#m)'/><rect width='16' height='16' mask='url(#m)'/>"
        ),
    ] {
        let tree = tree(&body);
        assert_eq!(
            run(&tree, flood_cost()),
            Err(RenderError::FilterBudgetExceeded)
        );
        assert!(run(&tree, u64::MAX).is_ok());
    }
}
#[test]
fn svg_images_do_not_start_fresh_budgets() {
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><defs>{FLOOD}</defs>{RECT}</svg>"
    );
    let encoded: String = svg.bytes().map(|b| format!("%{b:02X}")).collect();
    let image = format!("<image width='16' height='16' href='data:image/svg+xml,{encoded}'/>");
    let tree = tree(&format!("{image}{image}"));
    assert_eq!(
        run(&tree, flood_cost()),
        Err(RenderError::FilterBudgetExceeded)
    );
    assert!(run(&tree, 2 * flood_cost()).is_ok());
}
#[test]
fn feimage_local_context_keeps_the_outer_quota() {
    let body = format!(
        "<defs>{FLOOD}<g id='source'>{RECT}</g><filter id='outer' x='0' y='0' width='100%' height='100%'><feImage xlink:href='#source'/><feImage xlink:href='#source'/></filter></defs><rect width='16' height='16' filter='url(#outer)'/>"
    );
    let tree = tree(&body);
    let primitive = tree
        .filters()
        .iter()
        .flat_map(|f| f.primitives())
        .find(|p| matches!(p.kind(), usvg::filter::Kind::Image(_)))
        .unwrap();
    let image_cost = crate::filter::work_budget::cost(
        primitive.kind(),
        16,
        16,
        tiny_skia::Transform::identity(),
    )
    .unwrap();
    let budget = 4096 + 2 * image_cost + flood_cost();
    assert_eq!(run(&tree, budget), Err(RenderError::FilterBudgetExceeded));
    assert!(run(&tree, budget + flood_cost()).is_ok());
}
fn primitive_cost(xml: &str, w: u32, h: u32) -> u64 {
    let tree = tree(&format!(
        "<defs><filter id='f' x='0' y='0' width='100%' height='100%'>{xml}</filter></defs>{RECT}"
    ));
    assert_eq!(tree.filters()[0].primitives().len(), 1);
    crate::filter::work_budget::cost(
        tree.filters()[0].primitives()[0].kind(),
        w,
        h,
        tiny_skia::Transform::identity(),
    )
    .unwrap()
}
#[test]
fn parameters_and_narrow_wrap_loops_change_cost() {
    let turbulence = |n| {
        primitive_cost(
            &format!("<feTurbulence baseFrequency='.1' numOctaves='{n}'/>"),
            16,
            16,
        )
    };
    assert!(turbulence(9) > turbulence(1));
    let kernel = "0 0 0 0 1 0 0 0 0";
    let conv = |edge| {
        format!(
            "<feConvolveMatrix order='3' kernelMatrix='{kernel}' targetX='2' targetY='2' edgeMode='{edge}'/>"
        )
    };
    assert!(primitive_cost(&conv("wrap"), 1, 1) > primitive_cost(&conv("duplicate"), 1, 1));
    let merge = |nodes| {
        format!(
            "<feMerge>{}</feMerge>",
            "<feMergeNode in='SourceGraphic'/>".repeat(nodes)
        )
    };
    assert!(primitive_cost(&merge(4), 16, 16) > primitive_cost(&merge(1), 16, 16));
    assert!(
        primitive_cost("<feGaussianBlur stdDeviation='1e30'/>", 1, 1) > 100_000,
        "joint blur padding must not be charged as a single original pixel"
    );
}
#[test]
fn tiny_primitive_subregions_do_not_discount_whole_source_work() {
    let tree = tree(&format!(
        "<defs><filter id='f' x='0' y='0' width='100%' height='100%'><feFlood width='1%' height='1%'/></filter></defs>{RECT}"
    ));
    assert_eq!(
        run(&tree, flood_cost() - 1),
        Err(RenderError::FilterBudgetExceeded)
    );
}
