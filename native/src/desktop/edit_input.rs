//! 宿主手势适配；编辑模型只接收图像坐标，失焦和捕获丢失取消未提交草稿。
use super::canvas::Canvas;
use crate::{editor::Tool, viewport::Point};

#[cfg(test)]
#[path = "crop_input_tests.rs"]
mod crop_tests;

impl Canvas {
    /// 活动手势期间保持 viewport 不变，避免松手坐标与预览使用不同变换。
    pub(super) fn blocks_wheel(&self) -> bool {
        self.editor.draft().is_some()
            || self.erase_gesture.is_some()
            || self.selection_drag.is_some()
            || self.marquee.is_some()
            || self.path_drag.is_some()
            || self.scale_drag.is_some()
            || self.rotation_drag.is_some()
            || self.crop_drag.is_some()
    }
    pub fn pointer_down(&mut self, point: Point) -> bool {
        self.pointer_down_at(point, self.motion_epoch.elapsed().as_secs_f64() * 1000.0)
    }

    pub fn pointer_down_at(&mut self, point: Point, time: f64) -> bool {
        self.last_pointer_time = time;
        self.cancel_gesture();
        if self.crop_mode {
            return self.begin_crop(point);
        }
        if self.tool == Tool::Eraser {
            return self.begin_erase(point);
        }
        if self.tool == Tool::Select {
            // 原有端点/延伸柄优先，避免通用变换柄抢占专用路径编辑入口。
            if self.selected_index().is_some() && self.begin_path_handle(point) {
                return true;
            }
            if self.begin_rotation_handle(point) {
                return true;
            }
            if self.begin_scale_handle(point) {
                return true;
            }
            if self.begin_group_drag(point) {
                return true;
            }
            let was_group = self
                .selected_group()
                .is_some_and(|indices| indices.len() > 1);
            self.multi_selection = None;
            if was_group {
                self.selected = None;
            }
            let Some(point) = self.image_point(point, false) else {
                self.selected = None;
                return true;
            };
            match unsafe {
                super::selection_hit::pick(
                    &self.factory,
                    &self.editor,
                    point,
                    5.0 / self.viewport.scale,
                )
            } {
                Ok(index) => self.selected = index.map(|index| (index, self.editor.revision())),
                Err(error) => {
                    self.selected = None;
                    eprintln!("native-selection: {error}");
                }
            }
            self.selection_drag = self.selected.map(|_| (point, Point::default()));
            if self.selected.is_none() {
                self.marquee = Some(super::marquee::Marquee::new(point, &self.editor));
            }
            self.selection_effect_preview = self.selected_index().and_then(|index| {
                let stroke = &self.editor.elements()[index];
                matches!(stroke.tool, Tool::Blur | Tool::Mosaic).then(|| stroke.clone())
            });
            return true;
        }
        if self.tool == Tool::Pan {
            self.drag = Some(point);
            return true;
        }
        let Some(image_point) = self.image_point(point, false) else {
            return false;
        };
        if self.tool == Tool::Blur {
            let Some((source, ready, frame, base_projection)) = &self.blur_ready else {
                return false;
            };
            if ready.strength() != self.tool_profiles.blur_strength()
                || *frame != self.editor.annotation_frame()
                || *base_projection != self.editor.raster_base_projection()
            {
                return false;
            }
            if !self
                .editor
                .layers()
                .is_some_and(|layers| source.same_storage(layers))
            {
                return false;
            }
            return match self.editor.begin_blur(image_point, ready.clone()) {
                Ok(started) => {
                    if started {
                        self.draft_pointer = Some(point);
                    }
                    started
                }
                Err(error) => {
                    eprintln!("native-blur: {error:?}");
                    false
                }
            };
        }
        if self.tool == Tool::Serial {
            let radius = self.tool_profiles.serial_radius() as f32;
            let fill = self.editor.style().fill();
            return match self.editor.insert_serial_with_fill(
                image_point,
                radius,
                self.tool_profiles.serial_font_family(),
                fill,
            ) {
                Ok(index) => index.is_some(),
                Err(error) => {
                    eprintln!("native-serial: {error:?}");
                    false
                }
            };
        }
        match self.editor.begin_timed(self.tool, image_point, time) {
            Ok(started) => {
                if started {
                    self.draft_pointer = Some(point);
                }
                started
            }
            Err(error) => {
                eprintln!("native-edit: {error:?}");
                false
            }
        }
    }

    #[cfg(test)]
    pub fn pointer_move(&mut self, point: Point) -> bool {
        self.pointer_move_modified(point, false, false)
    }

    #[cfg(test)]
    pub fn pointer_move_modified(&mut self, point: Point, shift: bool, control: bool) -> bool {
        self.pointer_move_at(
            point,
            shift,
            control,
            self.motion_epoch.elapsed().as_secs_f64() * 1000.0,
        )
    }

    pub fn pointer_move_at(&mut self, point: Point, shift: bool, control: bool, time: f64) -> bool {
        self.last_pointer_time = time;
        if self.marquee.is_some() {
            return self.move_marquee(point);
        }
        if self.crop_drag.is_some() {
            return self.move_crop(point);
        }
        if self.scale_drag.is_some() {
            return self.move_scale_handle(point, shift);
        }
        if self.rotation_drag.is_some() {
            return self.move_rotation_handle(point);
        }
        if self.erase_gesture.is_some() {
            return self.extend_erase(point);
        }
        if self.path_drag.is_some() {
            return self.move_path_handle_preview(point);
        }
        if self.editor.draft().is_some() {
            self.draft_pointer = Some(point);
        }
        if let Some((start, previous)) = self.selection_drag {
            if !self.group_preview.is_empty() {
                return self.move_group_drag(point, start, previous);
            }
            if self.selected_index().is_none() {
                self.selection_drag = None;
                self.selection_effect_preview = None;
                return true;
            }
            let point = self.viewport.image_point(point);
            let offset = self.move_axis.constrain(Point {
                x: point.x - start.x,
                y: point.y - start.y,
            });
            if !offset.x.is_finite() || !offset.y.is_finite() {
                return false;
            }
            if let Some(index) = self.selected_index() {
                if let Some(preview) = &mut self.selection_effect_preview {
                    if preview
                        .update_translation_preview(&self.editor.elements()[index], offset)
                        .is_err()
                    {
                        return false;
                    }
                }
            }
            self.selection_drag = Some((start, offset));
            return offset != previous;
        }
        if let Some(previous) = self.drag {
            self.viewport.pan(Point {
                x: point.x - previous.x,
                y: point.y - previous.y,
            });
            self.drag = Some(point);
            return point != previous;
        }
        let Some(mut image_point) = self.image_point(point, true) else {
            return false;
        };
        let raw_point = image_point;
        if let Some(draft) = self.editor.draft() {
            if matches!(draft.tool, Tool::Line | Tool::Arrow) {
                image_point = crate::linear_constraint::configured_endpoint(
                    draft.points[0],
                    image_point,
                    shift,
                    control,
                    self.tool_profiles.angle_snap(draft.tool),
                );
            } else if shift
                && matches!(
                    draft.tool,
                    Tool::Brush | Tool::Highlighter | Tool::Paintbrush
                )
            {
                image_point =
                    crate::linear_constraint::endpoint(draft.points[0], image_point, true, false);
            } else if matches!(
                draft.tool,
                Tool::Rectangle
                    | Tool::RoundedRectangle
                    | Tool::Ellipse
                    | Tool::Triangle
                    | Tool::Polygon
            ) {
                if let Some(image) = &self.image {
                    image_point = self.tool_profiles.shapes.endpoint(
                        draft.tool,
                        draft.points[0],
                        self.viewport.image_point(point),
                        Point {
                            x: image.width as f32,
                            y: image.height as f32,
                        },
                        shift,
                        control,
                    );
                }
            }
        }
        match self
            .editor
            .extend_timed(image_point, shift, time, raw_point)
        {
            Ok(changed) => changed,
            Err(error) => {
                eprintln!("native-edit: {error:?}; draft cancelled");
                self.editor.cancel()
            }
        }
    }

    pub fn pointer_up(&mut self, point: Point) -> bool {
        self.pointer_up_modified(point, false, false)
    }

    pub fn pointer_up_modified(&mut self, point: Point, shift: bool, control: bool) -> bool {
        self.pointer_up_at(
            point,
            shift,
            control,
            self.motion_epoch.elapsed().as_secs_f64() * 1000.0,
        )
    }

    pub fn pointer_up_at(&mut self, point: Point, shift: bool, control: bool, time: f64) -> bool {
        if self.marquee.is_some() {
            self.move_marquee(point);
            self.finish_marquee();
            return true;
        }
        if self.crop_drag.is_some() {
            self.move_crop(point);
            return self.finish_crop();
        }
        if self.scale_drag.is_some() {
            self.move_scale_handle(point, shift);
            self.finish_scale_handle();
            return true;
        }
        if self.rotation_drag.is_some() {
            self.move_rotation_handle(point);
            self.commit_rotation_handle();
            return true;
        }
        if self.erase_gesture.is_some() {
            self.extend_erase(point);
            self.erase_pending = self.erase_gesture.take();
            return true;
        }
        if self.path_drag.is_some() {
            self.move_path_handle_preview(point);
            self.commit_path_handle();
            return true;
        }
        let changed = self.pointer_move_at(point, shift, control, time);
        if let Some((_, offset)) = self.selection_drag.take() {
            if !self.group_preview.is_empty() {
                self.commit_group_drag(offset);
                return true;
            }
            self.selection_effect_preview = None;
            if let Some(index) = self.selected_index() {
                match self.editor.translate(index, offset) {
                    Ok(true) => {
                        self.selected = Some((index, self.editor.revision()));
                        return true;
                    }
                    Ok(false) => {}
                    Err(error) => eprintln!("native-move: {error:?}; preview discarded"),
                }
            }
            return changed || offset != Point::default();
        }
        self.drag = None;
        self.draft_pointer = None;
        self.editor
            .commit_smoothed(self.tool_profiles.smoothing(self.tool) as f32)
            || changed
    }

    pub fn cancel_gesture(&mut self) -> bool {
        let marquee = self.marquee.take().is_some();
        let cropped = self.crop_drag.take().is_some();
        let pending_scale = self.scale_pending.take().is_some();
        let scaled = self.scale_drag.take().is_some() | pending_scale;
        if pending_scale {
            self.clear_scale_display();
        }
        let rotated = self.rotation_drag.take().is_some();
        let erased = self.erase_gesture.take().is_some() | self.erase_pending.take().is_some();
        self.selection_effect_preview = None;
        if self.scale_display_revision.is_none() {
            self.group_preview = Vec::new();
        }
        self.drag = None;
        self.draft_pointer = None;
        let handle = self.path_drag.take().is_some();
        let moved = self.selection_drag.take().is_some() || handle;
        self.editor.cancel() || moved || erased || rotated || scaled || cropped || marquee
    }

    /// Shift 可将自由笔迹草稿收束为直线；无 Shift 的按键不追加采样点。
    pub fn refresh_constraints(&mut self, shift: bool, control: bool) -> bool {
        let Some(draft) = self.editor.draft() else {
            return false;
        };
        if !shift
            && matches!(
                draft.tool,
                Tool::Brush | Tool::Highlighter | Tool::Paintbrush
            )
        {
            return false;
        }
        let Some(point) = self.draft_pointer else {
            return false;
        };
        self.pointer_move_at(point, shift, control, self.last_pointer_time)
    }

    pub fn select_tool(&mut self, tool: Tool) -> bool {
        let changed = self.cancel_gesture() || self.tool != tool || self.crop_mode;
        self.crop_mode = false;
        let style = self
            .tool_profiles
            .switch(self.tool, tool, self.editor.style());
        self.editor.set_style(style);
        self.tool = tool;
        self.selected = None;
        self.multi_selection = None;
        changed
    }

    pub(super) fn image_point(&self, point: Point, clamp: bool) -> Option<Point> {
        let image = self.image.as_ref()?;
        let point = self.viewport.image_point(point);
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let (left, top, right, bottom) = self.editor.annotation_frame().map_or(
            (0.0, 0.0, image.width as f32, image.height as f32),
            |r| {
                (
                    r.x as f32,
                    r.y as f32,
                    (r.x + r.width) as f32,
                    (r.y + r.height) as f32,
                )
            },
        );
        if !clamp && (point.x < left || point.y < top || point.x > right || point.y > bottom) {
            return None;
        }
        Some(Point {
            x: point.x.clamp(left, right),
            y: point.y.clamp(top, bottom),
        })
    }
}
