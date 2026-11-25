//! Vertical line hits reuse actual DirectWrite layout, preserving column coordinates rather than guessing the next line from character counts.
use super::*;
impl Layout {
    pub(crate) fn point_caret(&self, point: [f32; 2]) -> Result<[f32; 2]> {
        if point
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 16_777_216.)
        {
            return Err(invalid());
        }
        let (mut trailing, mut inside) = (BOOL::default(), BOOL::default());
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native
                .HitTestPoint(point[0], point[1], &mut trailing, &mut inside, &mut hit)?;
        }
        self.normalize_trimmed_hit(&mut hit)?;
        cluster_bounds(&hit)?;
        let right = trailing.as_bool() != (hit.bidiLevel % 2 != 0);
        Ok([
            hit.left + if right { hit.width } else { 0. },
            hit.top + hit.height / 2.,
        ])
    }
    pub(crate) fn point_cluster_rect(&self, point: [f32; 2]) -> Result<crate::surface_clip::Rect> {
        if point
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 16_777_216.)
        {
            return Err(invalid());
        }
        let (mut trailing, mut inside) = (BOOL::default(), BOOL::default());
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native
                .HitTestPoint(point[0], point[1], &mut trailing, &mut inside, &mut hit)?;
        }
        self.normalize_trimmed_hit(&mut hit)?;
        cluster_bounds(&hit)
    }
    pub fn vertical_text(
        &self,
        position: u32,
        down: bool,
        preferred: Option<[f32; 2]>,
    ) -> Result<(u32, [f32; 2])> {
        self.vertical_text_step(position, down, preferred)
            .map(|(position, point, _)| (position, point))
    }
    /// moved means entering an adjacent laid-out line; soft wraps may change lines without changing UTF-16 offsets.
    pub(crate) fn vertical_text_step(
        &self,
        position: u32,
        down: bool,
        preferred: Option<[f32; 2]>,
    ) -> Result<(u32, [f32; 2], bool)> {
        self.validate_range(position..position)?;
        if preferred.is_some_and(|point| {
            point
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 16_777_216.)
        }) {
            return Err(invalid());
        }
        if self.text_len() == 0 {
            return Ok((0, preferred.unwrap_or([0.; 2]), false));
        }
        let (mut x, mut y) = (0., 0.);
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            if let Some(point) = preferred {
                let (mut trailing, mut inside) = (BOOL::default(), BOOL::default());
                self.native.HitTestPoint(
                    point[0],
                    point[1],
                    &mut trailing,
                    &mut inside,
                    &mut hit,
                )?;
                x = point[0];
            } else {
                self.native
                    .HitTestTextPosition(position, false, &mut x, &mut y, &mut hit)?;
            }
        }
        self.normalize_trimmed_hit(&mut hit)?;
        if !x.is_finite() || !hit.top.is_finite() || !hit.height.is_finite() || hit.height <= 0. {
            return Err(invalid());
        }
        if (!down && hit.top <= 0.) || (down && hit.top + hit.height >= self.metrics.height) {
            return Ok((
                position,
                preferred.unwrap_or([x, hit.top + hit.height / 2.]),
                false,
            ));
        }
        // Step just beyond the actual current-line boundary so DirectWrite selects the nearest cluster on the adjacent line.
        let epsilon = (hit.height * 0.001).min(0.25);
        let y = if down {
            hit.top + hit.height + epsilon
        } else {
            hit.top - epsilon
        };
        let next = self.hit_text([x, y])?;
        // Retain the actual target-line probe so a shared UTF-16 index at a soft-wrap end is not mistaken for the next line later.
        Ok((next.position, [x, y], true))
    }
}
