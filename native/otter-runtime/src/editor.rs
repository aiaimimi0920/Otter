//! Editing transactions in image coordinates; drafts stay outside history, and commits copy neither pixels nor existing annotations.
use crate::edit_history::{Entry, History};
use crate::stroke_style::{Color, StrokeStyle};
use crate::viewport::Point;

#[path = "editor_adopt.rs"]
pub mod adopt;
#[path = "editor_adoption.rs"]
pub mod adoption;
#[path = "editor_annotation_id.rs"]
pub mod annotation_id;
#[path = "editor_annotations.rs"]
pub mod annotations;
pub use annotations::AnnotationReplacement;
#[path = "editor_overlay_size.rs"]
pub mod overlay_size;
#[path = "editor_restore.rs"]
pub mod restore;
#[path = "editor_snapshot.rs"]
pub mod snapshot;
pub use adoption::{Adoption, AdoptionOutcome};
#[path = "editor_batch.rs"]
pub mod batch;
#[path = "editor_batch_style.rs"]
pub mod batch_style;
#[path = "editor_crop.rs"]
pub mod crop;
#[path = "editor_crop_refresh.rs"]
pub mod crop_refresh;
#[path = "editor_document_stamp.rs"]
pub mod document_stamp;
#[path = "editor_effect_rect.rs"]
pub mod effect_rect;
#[path = "editor_serial_arrow.rs"]
pub mod serial_arrow;
#[path = "editor_text_box_resize.rs"]
mod text_box_resize;
pub use document_stamp::DocumentStamp;
#[path = "editor_flip_geometry.rs"]
pub mod flip_geometry;
pub use crop::CropView;
pub use flip_geometry::TextFlipMetrics;
#[path = "editor_flip.rs"]
pub mod flip;
pub use flip::{FlipError, FlipPlan};
#[path = "editor_mirror.rs"]
pub mod mirror;
pub use mirror::MirrorState;
#[path = "editor_group_resize.rs"]
pub mod group_resize;
pub use group_resize::ResizePlan;
#[path = "editor_group_rotation.rs"]
pub mod group_rotation;
#[path = "editor_raster.rs"]
pub mod raster;
#[path = "editor_raster_refresh.rs"]
pub mod raster_refresh;
pub use raster_refresh::RasterRefreshPlan;
#[path = "editor_history_state.rs"]
pub mod history_state;
#[path = "editor_pin_history.rs"]
pub mod pin_history;
pub use pin_history::{PinChange, PinValue};
#[path = "editor_commit.rs"]
pub mod commit;
#[path = "editor_line_property.rs"]
pub mod line_property;
#[path = "editor_magnifier.rs"]
mod magnifier;
#[path = "editor_rasterize.rs"]
pub mod rasterize;
#[path = "editor_resize_preview.rs"]
pub mod resize_preview;
#[path = "editor_rotation_preview.rs"]
pub mod rotation_preview;
#[path = "editor_spotlight.rs"]
mod spotlight;
pub use magnifier::{Lens, LensShape};
#[path = "editor_stroke_style.rs"]
pub mod stroke_style;
#[path = "editor_visibility.rs"]
pub mod visibility;

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
    SpotRect,
    SpotRounded,
    SpotEllipse,
    Magnifier,
}

impl Tool {
    /// Match baseline composition: highlighters at the bottom, blur below mosaic, ordinary annotations above effects.
    /// Preserve document order within layers; hit queries must traverse in reverse.
    pub fn render_rank(self) -> u8 {
        match self {
            Self::Highlighter => 0,
            Self::Blur => 1,
            Self::Mosaic => 2,
            Self::SpotRect | Self::SpotRounded | Self::SpotEllipse => 3,
            Self::Magnifier => 4,
            _ => 5,
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
    pub effect_rect: bool,
    /// Local offset from the circle center in radii. Box geometry stays exactly two points.
    pub serial_arrow_tip: Option<Point>,
    pub lens: Option<Lens>,
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
    presentation_revision: u64,
    serial_count: u32,
    spotlight_percent: Option<u8>,
    elements: Vec<Stroke>,
    history: History,
    retained_points: usize,
    draft: Option<Stroke>,
    draft_motion: Option<crate::brush_dynamics::Motion>,
    // Sampling provenance for the current draft only; commits retain existing Highlighter representation without new persistence types.
    draft_brush_highlighter: bool,
    raster: Option<crate::erase_layers::Layers>,
    crop: Option<CropView>,
    // Restored documents' full logical bounds may be fractional; never infer from ceil-rounded pixel buffers.
    logical_size: Option<crate::crop_geometry::Size>,
    mirror: MirrorState,
    pending_pin: Option<PinValue>,
}

impl Editor {
    /// Include redo branches/drafts to avoid silently discarding temporarily invisible edits when opening new images.
    pub fn has_edit_state(&self) -> bool {
        !self.elements.is_empty() || !self.history.is_empty() || self.draft.is_some()
    }
    pub fn style(&self) -> StrokeStyle {
        self.style
    }
    /// Settings affect only the next gesture, not old annotations/current drafts or document history.
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
        self.draft_brush_highlighter = false;
        self.draft = Some(Stroke {
            annotation_id: annotation_id::AnnotationId::new()?,
            tool,
            points: vec![point],
            width: self.style.width(),
            color: self.style.color(),
            opacity: self.style.opacity(),
            fill: self.style.fill(),
            paint_alpha: self.style.paint_alpha(),
            corner_radius: if tool == Tool::SpotRounded && self.style.corner_radius() == 0.0 {
                12.0
            } else {
                self.style.corner_radius()
            },
            dash: self.style.dash(),
            sides: self.style.sides(),
            rotation: 0.0,
            text: None,
            blur: None,
            mosaic_cell: self.style.mosaic_cell(),
            effect_rect: false,
            serial_arrow_tip: None,
            lens: (tool == Tool::Magnifier).then(Lens::default),
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

    /// Temporary straight lines replace only current freehand drafts and preserve start points, without history/existing-annotation copies.
    pub fn extend_constrained(&mut self, point: Point, straight: bool) -> Result<bool, EditError> {
        validate(point)?;
        let Some(stroke) = &mut self.draft else {
            return Ok(false);
        };
        if stroke.effect_rect {
            return Ok(effect_rect::extend(stroke, point));
        }
        if stroke.tool == Tool::Serial {
            return stroke.extend_serial(point, straight);
        }
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
        let pencil = stroke.tool == Tool::Brush || self.draft_brush_highlighter;
        if (pencil || stroke.tool == Tool::Paintbrush) && !straight {
            let last = stroke.points.last().unwrap();
            if (point.x - last.x).hypot(point.y - last.y) < 0.75 {
                return Ok(false);
            }
            if pencil && stroke.points.len() >= crate::freehand::MAX_POINTS {
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

    pub fn cancel(&mut self) -> bool {
        self.draft_motion = None;
        self.draft_brush_highlighter = false;
        self.draft.take().is_some()
    }

    pub fn undo(&mut self) -> bool {
        self.pending_pin = None;
        if self.cancel() {
            return true;
        }
        if !self.history.undo(crate::edit_history::Target {
            elements: &mut self.elements,
            serial_count: &mut self.serial_count,
            raster: &mut self.raster,
            crop: &mut self.crop,
            logical_size: &mut self.logical_size,
            mirror: &mut self.mirror,
            pending_pin: &mut self.pending_pin,
            spotlight: &mut self.spotlight_percent,
        }) {
            return false;
        }
        self.commit_history_step();
        true
    }

    pub fn redo(&mut self) -> bool {
        self.pending_pin = None;
        if self.draft.is_some()
            || !self.history.redo(crate::edit_history::Target {
                elements: &mut self.elements,
                serial_count: &mut self.serial_count,
                raster: &mut self.raster,
                crop: &mut self.crop,
                logical_size: &mut self.logical_size,
                mirror: &mut self.mirror,
                pending_pin: &mut self.pending_pin,
                spotlight: &mut self.spotlight_percent,
            })
        {
            return false;
        }
        self.commit_history_step();
        true
    }

    /// Deletion records own removed objects and restore original order; reject committed-document changes while drafts exist.
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

    /// Style history retains fixed-size properties only, without cloning stroke points or consuming point budgets.
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

    /// Copy only moved annotations at commit; history retains original coordinates so undo needs no reverse floating-point operations.
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
        moved.reanchor_translated_lens(stroke)?;
        let original = std::mem::replace(&mut self.elements[index], moved);
        self.retained_points += original.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, original));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[path = "editor_resize.rs"]
pub mod resize;

#[path = "editor_duplicate.rs"]
pub mod duplicate;

#[path = "editor_bezier.rs"]
pub mod bezier;

#[path = "editor_blur.rs"]
pub mod blur;
#[path = "editor_effect_preview.rs"]
pub mod effect_preview;
pub use effect_preview::TranslationBounds;
#[path = "editor_line_endpoint.rs"]
pub mod line_endpoint;
#[path = "editor_serial.rs"]
pub mod serial;
#[path = "editor_shape_handle.rs"]
pub mod shape_handle;
#[path = "editor_text.rs"]
pub mod text;

#[path = "editor_brush_motion.rs"]
pub mod brush_motion;

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
