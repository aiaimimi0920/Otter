// Copyright 2026 the Otter Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Explicit filter-failure reporting without global state or a second render pass.
use std::cell::Cell;

/// A filter failed during rasterization; the caller must discard the destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    /// A filter region, result, or numeric operation could not be represented.
    FilterFailed,
    /// The cumulative filter work policy quota was exhausted.
    FilterBudgetExceeded,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::FilterFailed => "SVG filter rasterization failed",
            Self::FilterBudgetExceeded => "SVG filter work budget exceeded",
        })
    }
}
impl std::error::Error for RenderError {}

/// Renders with explicit filter failure reporting, including nested subroots.
///
/// On error the destination may contain partial content and must be discarded.
/// This is not a guarantee that all unsupported SVG features or allocations are
/// reported: it exposes filter execution failures that legacy `render` discards.
pub fn render_checked(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Result<(), RenderError> {
    render_with_filter_budget(tree, transform, pixmap, u64::MAX)
}

/// Renders with a cumulative weighted filter-work quota shared by all subroots.
///
/// Credits account for actual filter executions and raster sizes, not XML nodes.
/// They are policy units, not a wall-clock, total SVG work, or peak-memory bound.
/// On any error, discard the destination. Zero permits SVGs without filters.
pub fn render_with_filter_budget(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
    max_filter_work: u64,
) -> Result<(), RenderError> {
    let filter_failed = Cell::new(false);
    let filter_work = crate::filter::work_budget::Budget::new(max_filter_work);
    let ctx = crate::render::Context {
        max_bbox: crate::max_filter_bbox(pixmap.width(), pixmap.height()),
        filter_failed: &filter_failed,
        filter_work: &filter_work,
    };
    crate::render::render_nodes(tree.root(), &ctx, transform, pixmap);
    if filter_work.exhausted() {
        Err(RenderError::FilterBudgetExceeded)
    } else if filter_failed.get() {
        Err(RenderError::FilterFailed)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "checked_render_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "filter_work_tests.rs"]
mod work_tests;
