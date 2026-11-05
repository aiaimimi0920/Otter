//! 选中模糊的请求构造；增量与绝对数值共用目标、源图及待处理强度校验。
use super::canvas::Canvas;
use crate::{blur::Region, blur_job::Request, editor::Tool};

pub(super) fn current(canvas: &Canvas, previous: Option<&Request>) -> Option<f32> {
    if canvas.tool != Tool::Select {
        return None;
    }
    let index = canvas.selected_index()?;
    let old = canvas.editor.elements()[index].blur.as_ref()?.strength();
    Some(
        previous
            .filter(|request| {
                request.target == Some(index)
                    && request.stamp.matches(&canvas.editor)
                    && request.revision == canvas.editor.revision()
                    && canvas
                        .editor
                        .layers()
                        .is_some_and(|layers| request.source.same_storage(layers))
            })
            .map_or(old, |request| request.sigma),
    )
}
pub(super) fn increment(
    canvas: &Canvas,
    previous: Option<&Request>,
    increment: f32,
) -> Option<Request> {
    if !increment.is_finite() {
        return None;
    }
    let value = (current(canvas, previous)? + increment).clamp(1.0, 64.0);
    absolute(canvas, previous, value)
}
pub(super) fn absolute(canvas: &Canvas, previous: Option<&Request>, sigma: f32) -> Option<Request> {
    if !sigma.is_finite() || !(1.0..=64.0).contains(&sigma) || sigma == current(canvas, previous)? {
        return None;
    }
    let image = canvas.image.as_ref()?;
    Some(Request {
        stamp: canvas.editor.document_stamp(),
        target: Some(canvas.selected_index()?),
        annotation_frame: canvas.editor.annotation_frame(),
        base_projection: canvas.editor.raster_base_projection(),
        source: canvas.editor.layers()?.clone(),
        size: (image.width, image.height),
        area: Region {
            x: 0,
            y: 0,
            width: image.width,
            height: image.height,
        },
        sigma,
        revision: canvas.editor.revision(),
    })
}
