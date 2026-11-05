//! 工具属性与已提交标注分离；构造时约束数值，避免异常线宽进入 GPU。
#[path = "stroke_paint_alpha.rs"]
mod paint_alpha;
pub use paint_alpha::PaintAlpha;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DashPattern {
    #[default]
    Solid,
    Dash,
    DashDot,
}
impl DashPattern {
    /// 旧格式 dash-1/dash-2 的图像空间长度，不随线宽改变。
    pub fn segments(self) -> &'static [f32] {
        match self {
            Self::Solid => &[],
            Self::Dash => &[8.0, 4.0],
            Self::DashDot => &[4.0, 2.0, 1.0, 2.0],
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Solid => Self::Dash,
            Self::Dash => Self::DashDot,
            Self::DashDot => Self::Solid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    width: f32,
    color: Color,
    opacity: f32,
    fill: Option<Color>,
    paint_alpha: PaintAlpha,
    corner_radius: f32,
    dash: DashPattern,
    sides: u8,
    arrow: bool,
    mosaic_cell: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: 3.0,
            color: Color(239, 68, 68),
            opacity: 1.0,
            fill: None,
            paint_alpha: PaintAlpha::default(),
            corner_radius: 0.0,
            dash: DashPattern::Solid,
            sides: 6,
            arrow: false,
            mosaic_cell: 12.0,
        }
    }
}

impl StrokeStyle {
    pub fn mosaic_cell(self) -> f32 {
        self.mosaic_cell
    }
    pub fn with_mosaic_cell(self, cell: u8) -> Option<Self> {
        if !(2..=64).contains(&cell) {
            return None;
        }
        self.with_mosaic_cell_value(cell as f32)
    }
    // 输入控件限制 2–64，但变换必须保留小数及超出控件范围的有效尺寸。
    pub(crate) fn with_mosaic_cell_value(mut self, cell: f32) -> Option<Self> {
        if !cell.is_finite() || cell <= 0.0 || cell > 1_000_000.0 {
            return None;
        }
        self.mosaic_cell = cell;
        Some(self)
    }
    pub fn arrow(self) -> bool {
        self.arrow
    }
    pub fn with_arrow(mut self, enabled: bool) -> Self {
        self.arrow = enabled;
        self
    }
    pub fn new(width: f32, color: Color) -> Option<Self> {
        (width.is_finite() && (1.0..=256.0).contains(&width)).then_some(Self {
            width,
            color,
            ..Self::default()
        })
    }
    /// 恢复/变换域独立于控件域；wire缩放后仍有界，不饱和截断合法值。
    pub(crate) fn restored(width: f32, color: Color) -> Option<Self> {
        Self::default().with_color(color).with_width_value(width)
    }
    pub(crate) fn with_width_value(mut self, width: f32) -> Option<Self> {
        if !width.is_finite() || !(0.0..=1_000_000.0).contains(&width) {
            return None;
        }
        self.width = width;
        Some(self)
    }
    pub(crate) fn with_corner_radius_value(mut self, radius: f32) -> Option<Self> {
        if !radius.is_finite() || !(0.0..=1_000_000.0).contains(&radius) {
            return None;
        }
        self.corner_radius = radius;
        Some(self)
    }
    pub(crate) fn with_sides_value(mut self, sides: u8) -> Option<Self> {
        if !(3..=64).contains(&sides) {
            return None;
        }
        self.sides = sides;
        Some(self)
    }
    pub fn with_opacity(mut self, opacity: f32) -> Option<Self> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return None;
        }
        self.opacity = opacity;
        Some(self)
    }
    pub fn with_dash(mut self, dash: DashPattern) -> Self {
        self.dash = dash;
        self
    }
    pub fn dash(self) -> DashPattern {
        self.dash
    }
    pub fn sides(self) -> u8 {
        self.sides
    }
    pub fn with_sides(mut self, sides: u8) -> Self {
        self.sides = sides.clamp(3, 12);
        self
    }
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self.paint_alpha.opaque_stroke();
        self
    }
    pub fn with_width(mut self, width: f32) -> Option<Self> {
        if !width.is_finite() || !(1.0..=256.0).contains(&width) {
            return None;
        }
        self.width = width;
        Some(self)
    }
    pub fn with_fill(mut self, fill: Option<Color>) -> Self {
        self.fill = fill;
        self.paint_alpha.opaque_fill();
        self
    }
    pub fn fill(self) -> Option<Color> {
        self.fill
    }
    pub fn paint_alpha(self) -> PaintAlpha {
        self.paint_alpha
    }
    pub fn with_paint_alpha(mut self, alpha: PaintAlpha) -> Self {
        self.paint_alpha = alpha;
        self
    }
    pub fn with_corner_radius(mut self, radius: f32) -> Option<Self> {
        if !radius.is_finite() || !(0.0..=256.0).contains(&radius) {
            return None;
        }
        self.corner_radius = radius;
        Some(self)
    }
    pub fn corner_radius(self) -> f32 {
        self.corner_radius
    }
    pub fn opacity(self) -> f32 {
        self.opacity
    }
    pub fn width(self) -> f32 {
        self.width
    }
    pub fn color(self) -> Color {
        self.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_baseline_creation_settings() {
        // 固定基线 stickerEditingDefaults.ts，避免恢复为早期原生演示参数。
        let style = StrokeStyle::default();
        assert_eq!(style.width(), 3.0);
        assert_eq!(style.color(), Color(239, 68, 68));
        assert_eq!(style.opacity(), 1.0);
        assert_eq!(style.fill(), None);
        assert_eq!(style.corner_radius(), 0.0);
        assert_eq!(style.dash(), DashPattern::Solid);
        assert_eq!(style.sides(), 6);
        assert_eq!(StrokeStyle::new(3.0, style.color()).unwrap(), style);
    }
    #[test]
    fn property_updates_preserve_independent_settings() {
        let style = StrokeStyle::default()
            .with_fill(Some(Color(1, 2, 3)))
            .with_corner_radius(20.0)
            .unwrap()
            .with_opacity(0.5)
            .unwrap()
            .with_color(Color(4, 5, 6))
            .with_width(10.0)
            .unwrap();
        assert_eq!(style.fill(), Some(Color(1, 2, 3)));
        assert_eq!(style.corner_radius(), 20.0);
        assert_eq!(style.opacity(), 0.5);
        assert_eq!(style.color(), Color(4, 5, 6));
        assert_eq!(style.width(), 10.0);
        for radius in [-1.0, 257.0, f32::NAN] {
            assert!(style.with_corner_radius(radius).is_none());
        }
    }
    #[test]
    fn opacity_rejects_invalid_values_and_preserves_other_attributes() {
        let style = StrokeStyle::new(8.0, Color(1, 2, 3)).unwrap();
        for opacity in [-0.1, 1.1, f32::INFINITY, f32::NAN] {
            assert!(style.with_opacity(opacity).is_none());
        }
        for opacity in [0.0, 0.5, 1.0] {
            let changed = style.with_opacity(opacity).unwrap();
            assert_eq!(changed.opacity(), opacity);
            assert_eq!(changed.width(), style.width());
            assert_eq!(changed.color(), style.color());
        }
    }
    #[test]
    fn width_is_finite_positive_and_bounded() {
        for width in [0.0, -1.0, 257.0, f32::INFINITY, f32::NAN] {
            assert!(StrokeStyle::new(width, Color(0, 0, 0)).is_none());
        }
        for width in [1.0, 4.0, 256.0] {
            assert_eq!(
                StrokeStyle::new(width, Color(1, 2, 3)).unwrap().width(),
                width
            );
        }
    }
}
