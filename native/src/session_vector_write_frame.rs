//! Write visible-frame coordinates and matching view metadata; undo only import translation, without rescaling, clipping outside annotations, or changing screen position.
use super::*;
use crate::{crop_geometry::Rect, editor::CropView};

pub struct Visible {
    pub annotations: Vec<u8>,
    /// Window wrappers must match this logical visible frame; do not mix in old window dimensions directly.
    pub frame: Option<Rect>,
    pub crop: Option<CropView>,
}

pub fn encode_visible(
    editor: &Editor,
    original: Option<&RawValue>,
    baseline: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
) -> Result<Visible, Error> {
    let frame = editor.visible_frame();
    let origin = match frame {
        Some(frame) => {
            if ![frame.x, frame.y, frame.width, frame.height]
                .iter()
                .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
                || frame.width <= 0.0
                || frame.height <= 0.0
            {
                return Err(Error::Geometry);
            }
            Point {
                x: frame.x as f32,
                y: frame.y as f32,
            }
        }
        None => Point::default(),
    };
    Ok(Visible {
        annotations: encode_origin(editor, original, origin, baseline)?,
        frame,
        crop: editor.crop_view(),
    })
}

pub(super) fn translate(element: &mut Value, origin: Point) -> Result<(), Error> {
    if origin == Point::default() {
        return Ok(());
    }
    let object = element.as_object_mut().ok_or(Error::Shape)?;
    // Process only coordinates just encoded, without recursively translating same-named keys in unknown extension fields.
    coordinate(object, "x", origin.x)?;
    coordinate(object, "y", origin.y)?;
    if let Some(points) = object.get_mut("points") {
        for point in points.as_array_mut().ok_or(Error::Shape)? {
            let point = point.as_object_mut().ok_or(Error::Shape)?;
            coordinate(point, "x", origin.x)?;
            coordinate(point, "y", origin.y)?;
        }
    }
    Ok(())
}
fn coordinate(object: &mut Map<String, Value>, key: &str, offset: f32) -> Result<(), Error> {
    if let Some(value) = object.get_mut(key) {
        let number = value.as_f64().ok_or(Error::Geometry)? as f32;
        *value = finite(number - offset)?;
    }
    Ok(())
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_vector_write_frame_tests.rs"]
mod tests;
