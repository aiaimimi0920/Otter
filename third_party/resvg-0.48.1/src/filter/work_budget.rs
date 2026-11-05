// Copyright 2026 the Otter Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Monotonic policy credits for actual filter raster work, not a CPU-time claim.
use std::cell::Cell;
use usvg::filter::{EdgeMode, Kind};

pub(crate) struct Budget {
    remaining: Cell<u64>,
    exhausted: Cell<bool>,
}
impl Budget {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            remaining: Cell::new(limit),
            exhausted: Cell::new(false),
        }
    }
    pub(crate) fn exhausted(&self) -> bool {
        self.exhausted.get()
    }
    pub(crate) fn reserve(&self, cost: Option<u64>) -> Option<()> {
        if !self.exhausted.get() {
            if let Some(left) = cost.and_then(|cost| self.remaining.get().checked_sub(cost)) {
                self.remaining.set(left);
                return Some(());
            }
        }
        // Overflow is rejection, never a cheap wraparound or a refunded request.
        self.remaining.set(0);
        self.exhausted.set(true);
        None
    }
}

pub(crate) fn cost(kind: &Kind, width: u32, height: u32, ts: usvg::Transform) -> Option<u64> {
    let (w, h) = (u64::from(width), u64::from(height));
    if w == 0 || h == 0 {
        return None;
    }
    let area = w.checked_mul(h)?;
    // Input copies, color conversion, output, clipping, and fixed setup. These
    // are conservative policy weights, not measured instruction counts.
    let base = area.checked_mul(64)?.checked_add(4096)?;
    let extra = match kind {
        Kind::ConvolveMatrix(fe) => {
            let matrix = fe.matrix();
            let taps = u64::from(matrix.rows()).checked_mul(u64::from(matrix.columns()))?;
            // Referenced inputs can be smaller than the source raster. Bound the
            // negative-coordinate Wrap loops using a minimum axis length of one.
            let wraps = if fe.edge_mode() == EdgeMode::Wrap {
                u64::from(matrix.target_x()).checked_add(u64::from(matrix.target_y()))?
            } else {
                0
            };
            area.checked_mul(taps)?
                .checked_mul(4u64.checked_add(wraps)?)?
        }
        Kind::Turbulence(fe) => area
            .checked_mul(128)?
            .checked_mul(u64::from(fe.num_octaves()))?
            .checked_add(area.checked_mul(16)?)?
            .checked_add(65536)?,
        Kind::Merge(fe) => area
            .checked_mul(64)?
            .checked_mul(u64::try_from(fe.inputs().len()).ok()?)?,
        Kind::Morphology(_) => area.checked_mul(32)?,
        Kind::ComponentTransfer(_) => 262144,
        Kind::GaussianBlur(fe) => blur(fe.std_dev_x().get(), fe.std_dev_y().get(), ts, w, h)?,
        Kind::DropShadow(fe) => blur(fe.std_dev_x().get(), fe.std_dev_y().get(), ts, w, h)?
            .checked_add(area.checked_mul(64)?)?,
        Kind::DiffuseLighting(_) | Kind::SpecularLighting(_) => area.checked_mul(128)?,
        Kind::Blend(_)
        | Kind::Flood(_)
        | Kind::Offset(_)
        | Kind::Composite(_)
        | Kind::Tile(_)
        | Kind::Image(_)
        | Kind::ColorMatrix(_)
        | Kind::DisplacementMap(_) => 0,
    };
    base.checked_add(extra)
}

fn blur(x: f32, y: f32, ts: usvg::Transform, w: u64, h: u64) -> Option<u64> {
    let Some((x, y)) = super::resolve_std_dev(x, y, ts) else {
        return Some(0);
    };
    let area = w.checked_mul(h)?;
    if !x.is_finite() || !y.is_finite() {
        return area.checked_mul(400);
    }
    if x <= 8.0 && y <= 8.0 {
        let taps = |sigma: f64| {
            if sigma > 0.0 {
                2 * (3.0 * sigma).ceil() as u64 + 1
            } else {
                0
            }
        };
        let (kx, ky) = (taps(x), taps(y));
        let product = kx * ky;
        let samples = if x <= 4.0 && y <= 4.0 && kx > 0 && ky > 0 && product <= 28 {
            product
        } else {
            kx + ky
        };
        return area.checked_mul(samples.checked_mul(4)?.checked_add(4)?);
    }
    // Finite f32 sigma needs <=128 reductions; each reduced extent is <= the
    // original extent +2. Two radius<=12 blur axes each add <=26 pixels.
    // Bound all intermediate areas by (w+28)*(h+28), including padding. This
    // deliberately overcharges large-sigma blur rather than duplicating its
    // geometry planner or ignoring the extra pixels on very narrow images.
    let padded = w.checked_add(28)?.checked_mul(h.checked_add(28)?)?;
    padded
        .checked_mul(128 * 17 + 2 * (25 * 4 + 1))?
        .checked_add(area.checked_mul(16)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_and_overflow_are_sticky_without_refunds() {
        let budget = Budget::new(7);
        assert!(budget.reserve(Some(3)).is_some());
        assert!(budget.reserve(Some(4)).is_some());
        assert!(!budget.exhausted());
        assert!(budget.reserve(Some(1)).is_none());
        assert!(budget.reserve(Some(0)).is_none());
        let budget = Budget::new(u64::MAX);
        assert!(budget.reserve(None).is_none());
        assert!(budget.exhausted());
    }
}
