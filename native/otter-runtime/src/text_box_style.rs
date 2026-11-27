//! Geometry in document units. Fill, opacity and corner radius have one owner:
//! the existing stroke style. Legacy text retains its fixed four-unit inset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBox {
    width: f32,
    padding: f32,
}
impl TextBox {
    pub fn new(width: f32, padding: f32) -> Option<Self> {
        (width.is_finite()
            && padding.is_finite()
            && (1.0..=1_000_000.0).contains(&width)
            && (0.0..=100_000.0).contains(&padding)
            && width - 2.0 * padding >= 1.0)
            .then_some(Self { width, padding })
    }
    pub fn width(self) -> f32 {
        self.width
    }
    pub fn padding(self) -> f32 {
        self.padding
    }
    pub fn scaled(self, scale: f32) -> Option<Self> {
        Self::new(self.width * scale, self.padding * scale)
    }
}
