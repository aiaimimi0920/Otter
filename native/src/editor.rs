//! 图像坐标中的编辑事务；草稿不进入历史，提交不复制像素或已有标注。
use crate::edit_history::{Entry, History};
use crate::stroke_style::{Color, StrokeStyle};
use crate::viewport::Point;

#[path = "editor_adopt.rs"]
mod adopt;
#[path = "editor_adoption.rs"]
mod adoption;
#[path = "editor_annotation_id.rs"]
mod annotation_id;
#[path = "editor_annotations.rs"]
mod annotations;
pub use annotations::AnnotationReplacement;
#[path = "editor_overlay_size.rs"]
mod overlay_size;
#[path = "editor_restore.rs"]
mod restore;
#[path = "editor_snapshot.rs"]
mod snapshot;
pub use adoption::{Adoption, AdoptionOutcome};
#[path = "editor_batch.rs"]
mod batch;
#[path = "editor_batch_style.rs"]
mod batch_style;
#[path = "editor_crop.rs"]
mod crop;
#[path = "editor_crop_refresh.rs"]
mod crop_refresh;
#[path = "editor_document_stamp.rs"]
mod document_stamp;
pub(crate) use document_stamp::DocumentStamp;
#[path = "editor_flip_geometry.rs"]
mod flip_geometry;
pub use crop::CropView;
pub use flip_geometry::TextFlipMetrics;
#[path = "editor_flip.rs"]
mod flip;
pub use flip::{FlipError, FlipPlan};
#[path = "editor_group_resize.rs"]
mod group_resize;
pub use group_resize::ResizePlan;
#[path = "editor_group_rotation.rs"]
mod group_rotation;
#[path = "editor_raster.rs"]
mod raster;
#[path = "editor_raster_refresh.rs"]
mod raster_refresh;
pub use raster_refresh::RasterRefreshPlan;
#[path = "editor_rasterize.rs"]
mod rasterize;
#[path = "editor_resize_preview.rs"]
mod resize_preview;
#[path = "editor_rotation_preview.rs"]
mod rotation_preview;
#[path = "editor_stroke_style.rs"]
mod stroke_style;
#[path = "editor_visibility.rs"]
mod visibility;

const MAX_POINTS_PER_STROKE: usize = 65_536;
const MAX_TOTAL_POINTS: usize = 1_048_576;
const MAX_ELEMENTS: usize = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Pan,
    Select,
    Text,
    Serial,
    Mosaic,
    Blur,
    Eraser,
    Brush,
    Highlighter,
    Line,
    Arrow,
    Bezier,
    Polyline,
    Paintbrush,
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Triangle,
    Polygon,
}

impl Tool {
    /// 与主干合成一致：荧光笔底层，模糊在马赛克之下，普通标注在效果之上。
    /// 同层仍按文档顺序；命中查询必须使用逆序。
    pub fn render_rank(self) -> u8 {
        match self {
            Self::Highlighter => 0,
            Self::Blur => 1,
            Self::Mosaic => 2,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stroke {
    annotation_id: annotation_id::AnnotationId,
    pub tool: Tool,
    pub points: Vec<Point>,
    pub width: f32,
    pub color: Color,
    pub opacity: f32,
    pub fill: Option<Color>,
    pub paint_alpha: crate::stroke_style::PaintAlpha,
    pub corner_radius: f32,
    pub dash: crate::stroke_style::DashPattern,
    pub sides: u8,
    pub rotation: f32,
    pub arrow: bool,
    pub brush_weights: Vec<f32>,
    pub text: Option<crate::text_annotation::TextAnnotation>,
    pub mosaic_cell: f32,
    pub blur: Option<crate::blur_image::BlurImage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    IdentityUnavailable,
    InvalidPoint,
    BudgetExceeded,
}

#[derive(Default)]
pub struct Editor {
    identity: std::sync::Arc<()>,
    style: StrokeStyle,
    revision: u64,
    serial_count: u32,
    elements: Vec<Stroke>,
    history: History,
    retained_points: usize,
    draft: Option<Stroke>,
    draft_motion: Option<crate::brush_dynamics::Motion>,
    raster: Option<crate::erase_layers::Layers>,
    crop: Option<CropView>,
    // 恢复文档的完整逻辑框可含小数；不能由像素缓冲的 ceil 尺寸反推。
    logical_size: Option<crate::crop_geometry::Size>,
}

impl Editor {
    /// 包含重做分支和草稿，避免打开新图时静默丢弃暂时不可见的编辑内容。
    pub fn has_edit_state(&self) -> bool {
        !self.elements.is_empty() || !self.history.is_empty() || self.draft.is_some()
    }
    pub fn style(&self) -> StrokeStyle {
        self.style
    }
    /// 设置只影响下一次手势；不改旧标注或当前草稿，也不污染文档历史。
    pub fn set_style(&mut self, style: StrokeStyle) {
        self.style = style;
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn elements(&self) -> &[Stroke] {
        &self.elements
    }
    pub fn draft(&self) -> Option<&Stroke> {
        self.draft.as_ref()
    }

    pub fn begin(&mut self, tool: Tool, point: Point) -> Result<bool, EditError> {
        validate(point)?;
        if matches!(
            tool,
            Tool::Pan | Tool::Select | Tool::Text | Tool::Serial | Tool::Blur | Tool::Eraser
        ) || self.draft.is_some()
        {
            return Ok(false);
        }
        if self.elements.len() >= MAX_ELEMENTS || self.retained_points >= MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        self.draft = Some(Stroke {
            annotation_id: annotation_id::AnnotationId::new()?,
            tool,
            points: vec![point],
            width: self.style.width(),
            color: self.style.color(),
            opacity: self.style.opacity(),
            fill: self.style.fill(),
            paint_alpha: self.style.paint_alpha(),
            corner_radius: self.style.corner_radius(),
            dash: self.style.dash(),
            sides: self.style.sides(),
            rotation: 0.0,
            text: None,
            blur: None,
            mosaic_cell: self.style.mosaic_cell(),
            arrow: self.style.arrow(),
            brush_weights: if tool == Tool::Paintbrush {
                vec![0.85]
            } else {
                Vec::new()
            },
        });
        Ok(true)
    }

    pub fn extend(&mut self, point: Point) -> Result<bool, EditError> {
        self.extend_constrained(point, false)
    }

    /// 临时直线只替换当前自由笔迹草稿，保留起点；不复制历史或已有标注。
    pub fn extend_constrained(&mut self, point: Point, straight: bool) -> Result<bool, EditError> {
        validate(point)?;
        let Some(stroke) = &mut self.draft else {
            return Ok(false);
        };
        if stroke.tool == Tool::Bezier {
            let points = crate::bezier_geometry::create(stroke.points[0], point);
            for &control in &points {
                validate(control)?;
            }
            if self.retained_points + points.len() > MAX_TOTAL_POINTS {
                return Err(EditError::BudgetExceeded);
            }
            if stroke.points.as_slice() == points {
                return Ok(false);
            }
            stroke.points.clear();
            stroke.points.extend_from_slice(&points);
            return Ok(true);
        }
        if straight
            && matches!(
                stroke.tool,
                Tool::Brush | Tool::Highlighter | Tool::Paintbrush
            )
            && stroke.points.len() >= 2
        {
            let changed = stroke.points.len() != 2 || stroke.points[1] != point;
            stroke.points.truncate(2);
            stroke.points[1] = point;
            if stroke.tool == Tool::Paintbrush {
                let weight = *stroke.brush_weights.last().unwrap();
                stroke.brush_weights.truncate(2);
                stroke.brush_weights[1] = weight;
            }
            return Ok(changed);
        }
        if stroke.points.last() == Some(&point) {
            return Ok(false);
        }
        if matches!(stroke.tool, Tool::Brush | Tool::Paintbrush) && !straight {
            let last = stroke.points.last().unwrap();
            if (point.x - last.x).hypot(point.y - last.y) < 0.75 {
                return Ok(false);
            }
            if stroke.tool == Tool::Brush && stroke.points.len() >= crate::freehand::MAX_POINTS {
                if self.retained_points + stroke.points.len().div_ceil(2) + 1 > MAX_TOTAL_POINTS {
                    return Err(EditError::BudgetExceeded);
                }
                let mut index = 0usize;
                stroke.points.retain(|_| {
                    let keep = index.is_multiple_of(2);
                    index += 1;
                    keep
                });
            }
        }
        if !matches!(
            stroke.tool,
            Tool::Brush
                | Tool::Highlighter
                | Tool::Polyline
                | Tool::Paintbrush
                | Tool::Mosaic
                | Tool::Blur
        ) && stroke.points.len() == 2
        {
            stroke.points[1] = point;
            return Ok(true);
        }
        if stroke.points.len() >= MAX_POINTS_PER_STROKE
            || self.retained_points + stroke.points.len() >= MAX_TOTAL_POINTS
        {
            return Err(EditError::BudgetExceeded);
        }
        stroke.points.push(point);
        if stroke.tool == Tool::Paintbrush {
            stroke
                .brush_weights
                .push(*stroke.brush_weights.last().unwrap());
        }
        Ok(true)
    }

    pub fn commit(&mut self) -> bool {
        self.commit_smoothed(0.0)
    }

    pub fn commit_smoothed(&mut self, smoothing: f32) -> bool {
        self.draft_motion = None;
        let Some(mut stroke) = self.draft.take() else {
            return false;
        };
        // 单击画笔保留一个点；零长度线和零面积形状不制造历史。
        if stroke.tool == Tool::Polyline && stroke.points.len() < 2 {
            return false;
        }
        if !matches!(
            stroke.tool,
            Tool::Brush
                | Tool::Highlighter
                | Tool::Polyline
                | Tool::Paintbrush
                | Tool::Mosaic
                | Tool::Blur
        ) && (stroke.points.len() < 2 || stroke.points.first() == stroke.points.last())
        {
            return false;
        }
        if matches!(
            stroke.tool,
            Tool::Rectangle
                | Tool::RoundedRectangle
                | Tool::Ellipse
                | Tool::Triangle
                | Tool::Polygon
        ) && (stroke.points[0].x == stroke.points[1].x
            || stroke.points[0].y == stroke.points[1].y)
        {
            return false;
        }
        if matches!(stroke.tool, Tool::Brush | Tool::Paintbrush) {
            crate::freehand::smooth(&mut stroke.points, smoothing);
        }
        self.retained_points += stroke.points.len();
        let index = self.elements.len();
        self.elements.push(stroke);
        self.retained_points -= self.history.record(Entry::added(index));
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn cancel(&mut self) -> bool {
        self.draft_motion = None;
        self.draft.take().is_some()
    }

    pub fn undo(&mut self) -> bool {
        if self.cancel() {
            return true;
        }
        if !self.history.undo(
            &mut self.elements,
            &mut self.serial_count,
            &mut self.raster,
            &mut self.crop,
            &mut self.logical_size,
        ) {
            return false;
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn redo(&mut self) -> bool {
        if self.draft.is_some()
            || !self.history.redo(
                &mut self.elements,
                &mut self.serial_count,
                &mut self.raster,
                &mut self.crop,
                &mut self.logical_size,
            )
        {
            return false;
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// 删除记录持有被移除对象，恢复原始顺序；草稿存在时拒绝修改已提交文档。
    pub fn remove(&mut self, index: usize) -> bool {
        if self.draft.is_some() || index >= self.elements.len() {
            return false;
        }
        let stroke = self.elements.remove(index);
        self.retained_points -= self.history.record(Entry::removed(index, stroke));
        self.revision = self.revision.wrapping_add(1);
        true
    }
    pub fn remove_last(&mut self) -> bool {
        self.elements
            .len()
            .checked_sub(1)
            .is_some_and(|index| self.remove(index))
    }

    /// 样式历史只保存固定大小的属性，不克隆笔迹点，也不消耗点预算。
    pub fn restyle(&mut self, index: usize, style: StrokeStyle) -> bool {
        if self.draft.is_some() {
            return false;
        }
        let Some(stroke) = self.elements.get_mut(index) else {
            return false;
        };
        if stroke.style() == style {
            return false;
        }
        let previous = stroke.apply_style(style);
        self.retained_points -= self.history.record(Entry::styled(index, previous));
        self.revision = self.revision.wrapping_add(1);
        true
    }
    pub fn restyle_last(&mut self) -> bool {
        self.elements
            .len()
            .checked_sub(1)
            .is_some_and(|index| self.restyle(index, self.style))
    }

    pub fn rotate(&mut self, index: usize, degrees: f32) -> Result<bool, EditError> {
        if !degrees.is_finite() || degrees.abs() > 3600.0 {
            return Err(EditError::InvalidPoint);
        }
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(stroke) = self.elements.get_mut(index) else {
            return Ok(false);
        };
        let angle = (stroke.rotation + degrees).rem_euclid(360.0);
        if angle == stroke.rotation {
            return Ok(false);
        }
        let previous = std::mem::replace(&mut stroke.rotation, angle);
        self.retained_points -= self.history.record(Entry::rotated(index, previous));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }

    /// 提交时只复制被移动标注；历史保存原坐标，撤销不靠反向浮点运算。
    pub fn translate(&mut self, index: usize, offset: Point) -> Result<bool, EditError> {
        validate(offset)?;
        if self.draft.is_some() || offset == Point::default() {
            return Ok(false);
        }
        let Some(stroke) = self.elements.get(index) else {
            return Ok(false);
        };
        if self.retained_points + stroke.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        for p in &stroke.points {
            validate(Point {
                x: p.x + offset.x,
                y: p.y + offset.y,
            })?;
        }
        let mut moved = stroke.clone();
        for point in &mut moved.points {
            point.x += offset.x;
            point.y += offset.y;
        }
        let original = std::mem::replace(&mut self.elements[index], moved);
        self.retained_points += original.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, original));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[path = "editor_resize.rs"]
mod resize;

#[path = "editor_duplicate.rs"]
mod duplicate;

#[path = "editor_bezier.rs"]
mod bezier;

#[path = "editor_blur.rs"]
mod blur;
#[path = "editor_effect_preview.rs"]
mod effect_preview;
#[path = "editor_line_endpoint.rs"]
mod line_endpoint;
#[path = "editor_serial.rs"]
mod serial;
#[path = "editor_shape_handle.rs"]
mod shape_handle;
#[path = "editor_text.rs"]
mod text;

#[path = "editor_brush_motion.rs"]
mod brush_motion;

fn validate(point: Point) -> Result<(), EditError> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x.abs() > 1_000_000.0
        || point.y.abs() > 1_000_000.0
    {
        Err(EditError::InvalidPoint)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "edit_history_tests.rs"]
mod history_tests;
