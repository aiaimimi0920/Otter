//! 描边与填充的独立透明度；整体 opacity 仍由标注组拥有，不折入任一颜色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintAlpha {
    stroke: f32,
    fill: f32,
}
impl Default for PaintAlpha {
    fn default() -> Self {
        Self {
            stroke: 1.0,
            fill: 1.0,
        }
    }
}
impl PaintAlpha {
    pub fn new(stroke: f32, fill: f32) -> Option<Self> {
        [stroke, fill]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .then_some(Self { stroke, fill })
    }
    pub fn stroke(self) -> f32 {
        self.stroke
    }
    pub fn fill(self) -> f32 {
        self.fill
    }
    pub(super) fn opaque_stroke(&mut self) {
        self.stroke = 1.0;
    }
    pub(super) fn opaque_fill(&mut self) {
        self.fill = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_alpha_is_finite_bounded_and_defaults_to_legacy_opaque() {
        assert_eq!(PaintAlpha::default(), PaintAlpha::new(1.0, 1.0).unwrap());
        for invalid in [-1.0, 1.01, f32::INFINITY, f32::NAN] {
            assert!(PaintAlpha::new(invalid, 0.5).is_none());
            assert!(PaintAlpha::new(0.5, invalid).is_none());
        }
        assert_eq!(PaintAlpha::new(0.0, 0.25).unwrap().fill(), 0.25);
    }
}
