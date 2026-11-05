//! 当前已实现属性的工具配置；只恢复该工具拥有的字段，不覆盖颜色与填充等共享编辑值。
use crate::{editor::Tool, stroke_style::StrokeStyle};

pub struct ToolProfiles {
    serial_style: StrokeStyle,
    before_serial: Option<StrokeStyle>,
    serial_radius: u16,
    blur_strength: f32,
    serial_font_family: String,
    pub text_font: crate::text_annotation::TextFont,
    pub shapes: crate::shape_options::ShapeOptions,
    styles: [StrokeStyle; 15],
    line_angle_snap: [bool; 2],
    brush_smoothing: u8,
    paintbrush_smoothing: u8,
}

impl Default for ToolProfiles {
    fn default() -> Self {
        let mut styles = [StrokeStyle::default(); 15];
        styles[14] = styles[14].with_width(20.0).unwrap();
        styles[11] = styles[11].with_width(16.0).unwrap();
        styles[12] = styles[12].with_width(28.0).unwrap();
        styles[13] = styles[13].with_width(28.0).unwrap();
        Self {
            serial_style: StrokeStyle::default()
                .with_fill(Some(crate::stroke_style::Color(0, 0, 0))),
            before_serial: None,
            serial_radius: 14,
            blur_strength: 8.0,
            serial_font_family: "微软雅黑".into(),
            text_font: crate::text_annotation::TextFont::default(),
            shapes: crate::shape_options::ShapeOptions::default(),
            styles,
            line_angle_snap: [false; 2],
            brush_smoothing: 0,
            paintbrush_smoothing: 50,
        }
    }
}

fn index(tool: Tool) -> Option<usize> {
    match tool {
        Tool::Brush => Some(0),
        Tool::Highlighter => Some(8),
        Tool::Bezier => Some(9),
        Tool::Polyline => Some(10),
        Tool::Paintbrush => Some(11),
        Tool::Mosaic => Some(12),
        Tool::Blur => Some(13),
        Tool::Eraser => Some(14),
        Tool::Line => Some(1),
        Tool::Arrow => Some(2),
        Tool::Rectangle => Some(3),
        Tool::RoundedRectangle => Some(4),
        Tool::Ellipse => Some(5),
        Tool::Triangle => Some(6),
        Tool::Polygon => Some(7),
        Tool::Pan | Tool::Select | Tool::Text | Tool::Serial => None,
    }
}

impl ToolProfiles {
    pub fn blur_strength(&self) -> f32 {
        self.blur_strength
    }
    pub fn set_blur_strength(&mut self, value: f32) -> bool {
        if !value.is_finite() || !(2.0..=64.0).contains(&value) {
            return false;
        }
        self.blur_strength = value;
        true
    }
    pub fn serial_font_family(&self) -> &str {
        &self.serial_font_family
    }
    pub fn set_serial_font_family(&mut self, family: &str) -> bool {
        if crate::text_annotation::TextFont::new(family, 16.0, false).is_none() {
            return false;
        }
        self.serial_font_family = family.to_owned();
        true
    }
    pub fn serial_radius(&self) -> u16 {
        self.serial_radius
    }
    pub fn set_serial_radius(&mut self, radius: u16) -> bool {
        if !(8..=999).contains(&radius) {
            return false;
        }
        self.serial_radius = radius;
        true
    }
    pub fn smoothing(&self, tool: Tool) -> u8 {
        if tool == Tool::Paintbrush {
            self.paintbrush_smoothing
        } else {
            self.brush_smoothing
        }
    }
    pub fn cycle_smoothing(&mut self, tool: Tool) {
        if tool == Tool::Paintbrush {
            self.paintbrush_smoothing = (self.paintbrush_smoothing + 25) % 125;
        } else if tool == Tool::Brush {
            self.cycle_brush_smoothing();
        }
    }
    pub fn brush_smoothing(&self) -> u8 {
        self.brush_smoothing
    }
    pub fn cycle_brush_smoothing(&mut self) {
        self.brush_smoothing = (self.brush_smoothing + 25) % 125;
    }
    pub fn angle_snap(&self, tool: Tool) -> bool {
        match tool {
            Tool::Line => self.line_angle_snap[0],
            Tool::Arrow => self.line_angle_snap[1],
            _ => false,
        }
    }

    pub fn toggle_angle_snap(&mut self, tool: Tool) -> bool {
        let slot = match tool {
            Tool::Line => 0,
            Tool::Arrow => 1,
            _ => return false,
        };
        self.line_angle_snap[slot] = !self.line_angle_snap[slot];
        true
    }

    pub fn switch(&mut self, previous: Tool, next: Tool, current: StrokeStyle) -> StrokeStyle {
        if previous == next {
            return current;
        }
        let current = if previous == Tool::Serial {
            self.serial_style = current;
            self.before_serial.take().unwrap_or(current)
        } else {
            current
        };
        if next == Tool::Serial {
            if let Some(index) = index(previous) {
                self.styles[index] = current;
            }
            self.before_serial = Some(current);
            return self.serial_style;
        }
        if let Some(index) = index(previous) {
            self.styles[index] = current;
        }
        let Some(index) = index(next) else {
            return current;
        };
        let stored = self.styles[index];
        // 所有存储值均来自通过校验的 StrokeStyle，恢复不发生失败或静默截断。
        let mut result = current.with_width(stored.width()).unwrap();
        result = result.with_arrow(stored.arrow());
        if next == Tool::Polyline {
            result = result.with_opacity(1.0).unwrap();
        }
        if matches!(next, Tool::Brush | Tool::Paintbrush) {
            result = result.with_opacity(stored.opacity()).unwrap();
        }
        if next == Tool::Highlighter {
            // 独立荧光笔默认使用完整 35% wash，不继承普通画笔的透明度。
            result = result.with_opacity(1.0).unwrap();
        }
        if matches!(
            next,
            Tool::Rectangle | Tool::RoundedRectangle | Tool::Triangle | Tool::Polygon
        ) {
            result = result.with_corner_radius(stored.corner_radius()).unwrap();
        }
        result = result.with_dash(stored.dash());
        if next == Tool::Mosaic {
            result = result.with_mosaic_cell_value(stored.mosaic_cell()).unwrap();
        }
        if next == Tool::Polygon {
            result = result.with_sides(stored.sides());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serial_font_is_independent_and_invalid_values_preserve_previous_family() {
        let mut profiles = ToolProfiles::default();
        let ordinary = profiles.text_font.clone();
        assert!(profiles.set_serial_font_family("Segoe UI"));
        for value in ["", " \t ", "bad\0font", "bad\nfont"] {
            assert!(!profiles.set_serial_font_family(value));
        }
        assert!(!profiles.set_serial_font_family(&"字".repeat(65)));
        assert_eq!(profiles.serial_font_family(), "Segoe UI");
        assert_eq!(profiles.text_font, ordinary);
        assert_eq!(profiles.serial_radius(), 14);
    }
    #[test]
    fn serial_profile_owns_colors_and_restores_previous_tool_style() {
        use crate::stroke_style::Color;
        let mut profiles = ToolProfiles::default();
        let brush = StrokeStyle::default()
            .with_width(9.0)
            .unwrap()
            .with_color(Color(1, 2, 3))
            .with_fill(None);
        let serial = profiles.switch(Tool::Brush, Tool::Serial, brush);
        assert_eq!(serial.fill(), Some(Color(0, 0, 0)));
        let serial = serial
            .with_color(Color(4, 5, 6))
            .with_fill(Some(Color(7, 8, 9)));
        assert!(profiles.set_serial_radius(30));
        assert!(!profiles.set_serial_radius(7));
        let restored = profiles.switch(Tool::Serial, Tool::Brush, serial);
        assert_eq!(restored, brush);
        let again = profiles.switch(Tool::Brush, Tool::Serial, restored);
        assert_eq!(again, serial);
        assert_eq!(profiles.serial_radius(), 30);
    }
    #[test]
    fn paintbrush_has_independent_baseline_width_opacity_and_smoothing() {
        let mut profiles = ToolProfiles::default();
        let paintbrush = profiles.switch(Tool::Pan, Tool::Paintbrush, StrokeStyle::default());
        assert_eq!(paintbrush.width(), 16.0);
        assert_eq!(profiles.smoothing(Tool::Paintbrush), 50);
        profiles.cycle_smoothing(Tool::Paintbrush);
        assert_eq!(profiles.smoothing(Tool::Paintbrush), 75);
        assert_eq!(profiles.smoothing(Tool::Brush), 0);
        let brush = profiles.switch(
            Tool::Paintbrush,
            Tool::Brush,
            paintbrush.with_opacity(0.5).unwrap(),
        );
        assert_eq!(brush.width(), 3.0);
        assert_eq!(brush.opacity(), 1.0);
        assert_eq!(
            profiles
                .switch(Tool::Brush, Tool::Paintbrush, brush)
                .opacity(),
            0.5
        );
    }
    #[test]
    fn curve_arrow_profile_does_not_leak_to_other_tools() {
        let mut profiles = ToolProfiles::default();
        let curve = StrokeStyle::default().with_arrow(true);
        let line = profiles.switch(Tool::Bezier, Tool::Line, curve);
        assert!(!line.arrow());
        assert!(profiles.switch(Tool::Line, Tool::Bezier, line).arrow());
    }
    #[test]
    fn angle_snap_is_owned_independently_by_line_and_arrow() {
        let mut profiles = ToolProfiles::default();
        assert!(!profiles.angle_snap(Tool::Line));
        assert!(!profiles.angle_snap(Tool::Arrow));
        assert!(profiles.toggle_angle_snap(Tool::Line));
        assert!(profiles.angle_snap(Tool::Line));
        assert!(!profiles.angle_snap(Tool::Arrow));
        assert!(!profiles.toggle_angle_snap(Tool::Rectangle));
        profiles.switch(Tool::Line, Tool::Arrow, StrokeStyle::default());
        profiles.switch(Tool::Arrow, Tool::Line, StrokeStyle::default());
        assert!(profiles.angle_snap(Tool::Line));
        assert!(profiles.toggle_angle_snap(Tool::Line));
        assert!(!profiles.angle_snap(Tool::Line));
    }
    use crate::stroke_style::{Color, DashPattern};
    #[test]
    fn every_supported_creation_tool_starts_with_baseline_defaults() {
        let mut profiles = ToolProfiles::default();
        for tool in [
            Tool::Brush,
            Tool::Highlighter,
            Tool::Line,
            Tool::Arrow,
            Tool::Bezier,
            Tool::Polyline,
            Tool::Rectangle,
            Tool::RoundedRectangle,
            Tool::Ellipse,
            Tool::Triangle,
            Tool::Polygon,
        ] {
            let style = profiles.switch(Tool::Pan, tool, StrokeStyle::default());
            assert_eq!(style, StrokeStyle::default(), "{tool:?}");
        }
    }
    #[test]
    fn switching_restores_owned_settings_without_overwriting_shared_colors() {
        let mut profiles = ToolProfiles::default();
        let brush = StrokeStyle::new(20.0, Color(1, 2, 3))
            .unwrap()
            .with_opacity(0.25)
            .unwrap();
        let rectangle = profiles
            .switch(Tool::Brush, Tool::Rectangle, brush)
            .with_width(8.0)
            .unwrap()
            .with_corner_radius(30.0)
            .unwrap()
            .with_dash(DashPattern::DashDot)
            .with_color(Color(4, 5, 6))
            .with_fill(Some(Color(7, 8, 9)));
        let restored = profiles.switch(Tool::Rectangle, Tool::Brush, rectangle);
        assert_eq!(restored.width(), 20.0);
        assert_eq!(restored.opacity(), 0.25);
        assert_eq!(restored.color(), rectangle.color());
        assert_eq!(restored.fill(), rectangle.fill());
        let restored = profiles.switch(Tool::Brush, Tool::Rectangle, restored);
        assert_eq!(restored.width(), 8.0);
        assert_eq!(restored.corner_radius(), 30.0);
        assert_eq!(restored.dash(), DashPattern::DashDot);
    }
    #[test]
    fn selection_and_pan_do_not_overwrite_creation_profiles() {
        let mut profiles = ToolProfiles::default();
        let line = StrokeStyle::default().with_width(9.0).unwrap();
        let selection = profiles.switch(Tool::Line, Tool::Select, line);
        let pan = profiles.switch(Tool::Select, Tool::Pan, selection.with_width(40.0).unwrap());
        assert_eq!(profiles.switch(Tool::Pan, Tool::Line, pan).width(), 9.0);
        assert_eq!(profiles.switch(Tool::Line, Tool::Line, line), line);
    }
    #[test]
    fn polygon_sides_are_not_replaced_by_other_shape_settings() {
        let mut profiles = ToolProfiles::default();
        let polygon = StrokeStyle::default().with_sides(9);
        let triangle = profiles
            .switch(Tool::Polygon, Tool::Triangle, polygon)
            .with_sides(4);
        assert_eq!(
            profiles
                .switch(Tool::Triangle, Tool::Polygon, triangle)
                .sides(),
            9
        );
    }
}
