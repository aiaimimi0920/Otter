//! Aspect-preserving Live fit geometry; drawing and input share the actual f32 destination rectangle without granting input-forwarding permission.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    source: [u32; 2],
    rect: [f32; 4],
    region: [f64; 4],
    mirrored: [bool; 2],
}
impl Viewport {
    pub fn fit(source: [u32; 2], destination: [u32; 2]) -> Option<Self> {
        Self::fit_content(source, destination, source.map(f64::from))
    }
    pub fn fit_content(source: [u32; 2], destination: [u32; 2], content: [f64; 2]) -> Option<Self> {
        if source.contains(&0) || destination.contains(&0) {
            return None;
        }
        if content.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return None;
        }
        let [width, height] = content.map(|value| value as f32);
        let [output_width, output_height] = destination.map(|value| value as f32);
        let scale = (output_width / width).min(output_height / height);
        let left = (output_width - width * scale) * 0.5;
        let top = (output_height - height * scale) * 0.5;
        let rect = [left, top, left + width * scale, top + height * scale];
        if rect[2] <= rect[0] || rect[3] <= rect[1] || rect.iter().any(|value| !value.is_finite()) {
            return None;
        }
        Some(Self {
            source,
            rect,
            region: [0.0, 0.0, 1.0, 1.0],
            mirrored: [false; 2],
        })
    }
    /// Separate the display box from the source sample region: input after Live crop/mirror edits still normalizes to the original captured frame.
    pub fn with_region(mut self, region: [f64; 4], mirrored: [bool; 2]) -> Self {
        self.region = region;
        self.mirrored = mirrored;
        self
    }
    pub fn destination(&self) -> [f32; 4] {
        self.rect
    }
    pub fn source_size(&self) -> [u32; 2] {
        self.source
    }
    /// Continuous coordinates used by the input transport; ordinary input rejects letterboxing, and only an already captured gesture may clamp.
    pub fn normalized(&self, point: [f64; 2], clamp: bool) -> Option<[f64; 2]> {
        let [left, top, right, bottom] = self.rect.map(f64::from);
        if !point.iter().all(|value| value.is_finite())
            || (!clamp
                && (point[0] < left || point[0] >= right || point[1] < top || point[1] >= bottom))
        {
            return None;
        }
        let local = [
            ((point[0] - left) / (right - left)).clamp(0.0, 1.0),
            ((point[1] - top) / (bottom - top)).clamp(0.0, 1.0),
        ];
        let mapped: [f64; 2] = std::array::from_fn(|axis| {
            let p = self.region[axis] + local[axis] * self.region[axis + 2];
            if self.mirrored[axis] {
                1.0 - p
            } else {
                p
            }
        });
        if !clamp && mapped.iter().any(|p| !(0.0..=1.0).contains(p)) {
            return None;
        }
        Some(mapped.map(|p| p.clamp(0.0, 1.0)))
    }
    /// Project positions in the original captured frame back onto the committed image; do not clamp carets outside the crop to the edge.
    /// Unlike half-open hit testing for new mouse input, text insertion points may lie on the image's right/bottom boundary.
    pub fn project(&self, point: [f64; 2]) -> Option<[f64; 2]> {
        if point
            .iter()
            .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
        {
            return None;
        }
        let local: [f64; 2] = std::array::from_fn(|axis| {
            let p = if self.mirrored[axis] {
                1.0 - point[axis]
            } else {
                point[axis]
            };
            (p - self.region[axis]) / self.region[axis + 2]
        });
        if local.iter().any(|p| !(0.0..=1.0).contains(p)) {
            return None;
        }
        Some(std::array::from_fn(|axis| {
            let start = f64::from(self.rect[axis]);
            start + local[axis] * (f64::from(self.rect[axis + 2]) - start)
        }))
    }
    /// New input accepts only the half-open image region; letterboxing and right/bottom boundaries cannot become in-source clicks.
    pub fn map(&self, point: [f64; 2]) -> Option<[u32; 2]> {
        let [left, top, right, bottom] = self.rect.map(f64::from);
        if !point.iter().all(|value| value.is_finite())
            || point[0] < left
            || point[1] < top
            || point[0] >= right
            || point[1] >= bottom
        {
            return None;
        }
        self.map_clamped(point)
    }
    /// Only for authorized drag/release with an existing pressed state; callers must not use this to allow new presses in letterboxing.
    /// Return frame pixels, not screen/client coordinates; the input transport must still validate target geometry and identity.
    pub fn map_clamped(&self, point: [f64; 2]) -> Option<[u32; 2]> {
        if !point.iter().all(|value| value.is_finite()) {
            return None;
        }
        let normalized = self.normalized(point, true)?;
        Some(std::array::from_fn(|axis| {
            ((normalized[axis] * f64::from(self.source[axis])).floor() as u32)
                .min(self.source[axis] - 1)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_source_letterbox_matches_paint_and_rejects_black_bars() {
        let view = Viewport::fit([200, 100], [400, 400]).unwrap();
        assert_eq!(view.destination(), [0.0, 100.0, 400.0, 300.0]);
        assert_eq!(view.map([200.0, 200.0]), Some([100, 50]));
        assert_eq!(view.normalized([200.0, 200.0], false), Some([0.5, 0.5]));
        assert_eq!(view.normalized([-1.0, 500.0], false), None);
        assert_eq!(view.normalized([-1.0, 500.0], true), Some([0.0, 1.0]));
        assert_eq!(view.map([0.0, 100.0]), Some([0, 0]));
        assert_eq!(view.map([399.99, 299.99]), Some([199, 99]));
        for point in [
            [200.0, 99.99],
            [200.0, 300.0],
            [-0.01, 200.0],
            [400.0, 200.0],
        ] {
            assert!(view.map(point).is_none());
        }
    }
    #[test]
    fn portrait_source_and_resize_change_mapping_without_stale_offsets() {
        let old = Viewport::fit([100, 200], [400, 400]).unwrap();
        assert_eq!(old.destination(), [100.0, 0.0, 300.0, 400.0]);
        assert_eq!(old.map([200.0, 200.0]), Some([50, 100]));
        let new = Viewport::fit([100, 200], [200, 200]).unwrap();
        assert_eq!(new.destination(), [50.0, 0.0, 150.0, 200.0]);
        assert!(new.map([200.0, 200.0]).is_none());
        assert_eq!(new.map([100.0, 100.0]), Some([50, 100]));
    }
    #[test]
    fn captured_release_can_clamp_but_invalid_coordinates_never_map() {
        let view = Viewport::fit([200, 100], [400, 400]).unwrap();
        assert_eq!(view.map_clamped([-50.0, 500.0]), Some([0, 99]));
        assert_eq!(view.map_clamped([400.0, 300.0]), Some([199, 99]));
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(view.map([value, 200.0]).is_none());
            assert!(view.map_clamped([200.0, value]).is_none());
        }
        assert!(Viewport::fit([0, 1], [1, 1]).is_none());
        assert!(Viewport::fit([1, 1], [1, 0]).is_none());
        assert_eq!(
            Viewport::fit([1, 1], [1, 1]).unwrap().map([0.5, 0.5]),
            Some([0, 0])
        );
    }
    #[test]
    fn source_projection_inverts_crop_mirror_and_letterbox_without_clamping() {
        for mirrored in [[false, false], [true, false], [false, true], [true, true]] {
            let view = Viewport::fit([200, 100], [400, 400])
                .unwrap()
                .with_region([0.25, 0.25, 0.5, 0.5], mirrored);
            for point in [[0.0, 100.0], [200.0, 200.0], [399.0, 299.0]] {
                let source = view.normalized(point, false).unwrap();
                let projected = view.project(source).unwrap();
                assert!((projected[0] - point[0]).abs() < 0.00001);
                assert!((projected[1] - point[1]).abs() < 0.00001);
            }
            assert_eq!(view.project([0.0, 0.0]), None);
            assert_eq!(view.project([1.0, 1.0]), None);
        }
        let view = Viewport::fit([200, 100], [400, 400]).unwrap();
        assert_eq!(view.project([0.0, 0.0]), Some([0.0, 100.0]));
        assert_eq!(view.project([1.0, 1.0]), Some([400.0, 300.0]));
        for invalid in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
            assert!(view.project([invalid, 0.5]).is_none());
        }
    }
}
