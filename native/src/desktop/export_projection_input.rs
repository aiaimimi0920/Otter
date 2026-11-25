//! Pending operations captured after host provenance validation; pixel dimensions do not prove logical bounds or materialization state.
use crate::crop_geometry::{Rect, Size};

/// Host-validated originals for natural-pixel restoration; new canonical data from ordinary saves cannot replace them.
#[derive(Clone)]
pub(in crate::desktop) struct Baseline {
    pub record: crate::session_source::Source,
    pub unit: crate::workspace_id::UnitId,
}

#[derive(Clone, Default)]
pub(in crate::desktop) struct Input {
    /// Complete Editor coordinate bounds, not current visible crop or natural-pixel dimensions.
    pub logical: Option<Size>,
    /// Original logical sourceSize of natural provenance; after nonuniform restoration it differs from current full Editor bounds.
    pub source_logical: Option<Size>,
    /// Selected pixels already cover the complete logical source frame, padding included.
    pub rasterized_source: bool,
    /// Interpret only natural-source history/layout, not target current crop, annotations, or appearance.
    pub baseline: Option<Baseline>,
    /// Only target-axis flips not yet materialized; already-flipped pixels must pass false.
    pub flip: (bool, bool),
    /// Historical erasure not yet included in natural-source baseline; current layers.base must not replay it.
    pub historical_erase: bool,
    /// Natural-source propagation placement not yet materialized; independent annotation coordinates are unaffected.
    pub content: bool,
    /// Replay only legacy pixel crops proven current by the host; false is required after reset.
    pub legacy_crop: bool,
    /// Valid post-flip region materialized by the current pixel owner; independent of content and preferred over old original crop.
    pub legacy_pixels: Option<Rect>,
}
