//! 形状创建的共享会话设置；不属于标注样式或撤销历史。
use crate::{editor::Tool, viewport::Point};

#[derive(Default)]
pub struct ShapeOptions {
    pub lock_aspect: bool,
    step_index: usize,
    ratio_index: usize,
    custom_ratio: Option<(String, f32)>,
    custom_step: Option<u16>,
}

const STEPS: [u16; 5] = [0, 5, 10, 20, 50];
const RATIOS: [(&str, f32); 5] = [
    ("1:1", 1.0),
    ("4:3", 4.0 / 3.0),
    ("3:4", 3.0 / 4.0),
    ("16:9", 16.0 / 9.0),
    ("9:16", 9.0 / 16.0),
];

impl ShapeOptions {
    pub fn supports(tool: Tool) -> bool {
        matches!(
            tool,
            Tool::Rectangle
                | Tool::RoundedRectangle
                | Tool::Ellipse
                | Tool::Triangle
                | Tool::Polygon
        )
    }
    pub fn supports_ratio(tool: Tool) -> bool {
        matches!(
            tool,
            Tool::Rectangle | Tool::RoundedRectangle | Tool::Ellipse
        )
    }
    pub fn step(&self) -> u16 {
        self.custom_step.unwrap_or(STEPS[self.step_index])
    }
    pub fn ratio_label(&self) -> &str {
        self.custom_ratio
            .as_ref()
            .map_or(RATIOS[self.ratio_index].0, |(text, _)| text.as_str())
    }
    pub fn set_ratio(&mut self, text: &str) -> bool {
        let Some(ratio) = crate::shape_value::ratio(text) else {
            return false;
        };
        self.custom_ratio = Some((text.trim().to_owned(), ratio));
        true
    }
    pub fn set_step(&mut self, text: &str) -> bool {
        let Some(step) = crate::shape_value::step(text) else {
            return false;
        };
        self.custom_step = Some(step);
        true
    }
    pub fn cycle_step(&mut self) {
        self.custom_step = None;
        self.step_index = (self.step_index + 1) % STEPS.len();
    }
    pub fn cycle_ratio(&mut self) {
        self.custom_ratio = None;
        self.ratio_index = (self.ratio_index + 1) % RATIOS.len();
    }
    pub fn endpoint(
        &self,
        tool: Tool,
        start: Point,
        current: Point,
        size: Point,
        shift: bool,
        control: bool,
    ) -> Point {
        let ratio = if !shift && Self::supports_ratio(tool) {
            self.custom_ratio
                .as_ref()
                .map_or(RATIOS[self.ratio_index].1, |(_, ratio)| *ratio)
        } else {
            1.0
        };
        let step = if self.step() > 0 {
            self.step() as f32
        } else if control {
            10.0
        } else {
            0.0
        };
        crate::shape_constraint::configured_endpoint(
            start,
            current,
            size,
            self.lock_aspect || shift,
            step,
            ratio,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_values_drive_geometry_and_invalid_input_preserves_settings() {
        let mut options = ShapeOptions {
            lock_aspect: true,
            ..Default::default()
        };
        assert!(options.set_ratio("2.5:1"));
        assert!(options.set_step("7"));
        assert!(!options.set_ratio("0:1"));
        assert!(!options.set_step("1000"));
        assert_eq!(options.ratio_label(), "2.5:1");
        assert_eq!(options.step(), 7);
        assert_eq!(
            options.endpoint(
                Tool::Rectangle,
                p(0.0, 0.0),
                p(30.0, 10.0),
                p(100.0, 100.0),
                false,
                false
            ),
            p(35.0, 14.0)
        );
        options.cycle_ratio();
        options.cycle_step();
        assert_eq!(options.ratio_label(), "4:3");
        assert_eq!(options.step(), 5);
    }
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    #[test]
    fn configured_ratio_is_shared_but_shift_and_polygon_use_square() {
        let mut options = ShapeOptions {
            lock_aspect: true,
            ..Default::default()
        };
        options.cycle_ratio();
        let endpoint = |tool, shift| {
            options.endpoint(
                tool,
                p(10.0, 10.0),
                p(50.0, 20.0),
                p(100.0, 100.0),
                shift,
                false,
            )
        };
        assert_eq!(endpoint(Tool::Rectangle, false), p(50.0, 40.0));
        assert_eq!(endpoint(Tool::Ellipse, false), p(50.0, 40.0));
        assert_eq!(endpoint(Tool::Rectangle, true), p(50.0, 50.0));
        assert_eq!(endpoint(Tool::Polygon, false), p(50.0, 50.0));
    }
    #[test]
    fn configured_step_overrides_control_and_cycles_off() {
        let mut options = ShapeOptions::default();
        let evaluate = |o: &ShapeOptions, control| {
            o.endpoint(
                Tool::Rectangle,
                p(0.0, 0.0),
                p(7.0, 17.0),
                p(100.0, 100.0),
                false,
                control,
            )
        };
        assert_eq!(evaluate(&options, true), p(10.0, 20.0));
        options.cycle_step();
        assert_eq!(evaluate(&options, true), p(5.0, 15.0));
        for _ in 0..4 {
            options.cycle_step();
        }
        assert_eq!(evaluate(&options, false), p(7.0, 17.0));
        for _ in 0..5 {
            options.cycle_ratio();
        }
        assert_eq!(options.ratio_label(), "1:1");
    }
    #[test]
    fn ratio_is_preserved_at_image_boundary_in_both_directions() {
        let result = crate::shape_constraint::configured_endpoint(
            p(80.0, 40.0),
            p(150.0, 90.0),
            p(100.0, 100.0),
            true,
            10.0,
            2.0,
        );
        assert_eq!(result, p(100.0, 50.0));
        let result = crate::shape_constraint::configured_endpoint(
            p(20.0, 40.0),
            p(-50.0, -20.0),
            p(100.0, 100.0),
            true,
            10.0,
            2.0,
        );
        assert_eq!(result, p(0.0, 30.0));
    }
}
