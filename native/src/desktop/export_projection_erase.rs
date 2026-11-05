//! 历史擦除有界逐笔适配到原擦除器，不复制完整edit/history或触及共享源像素。
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize)]
struct History<'a> {
    #[serde(rename = "contentEraseStrokes", default, borrow)]
    strokes: Option<&'a RawValue>,
}
fn strokes<'de: 'a, 'a, D: serde::Deserializer<'de>>(
    decoder: D,
) -> std::result::Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, 4096>(decoder)
}
#[derive(Deserialize)]
struct Stroke<'a> {
    #[serde(default, borrow)]
    width: Option<&'a RawValue>,
    #[serde(borrow, deserialize_with = "points")]
    points: Vec<&'a RawValue>,
}
fn points<'de: 'a, 'a, D: serde::Deserializer<'de>>(
    decoder: D,
) -> std::result::Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, 65_536>(decoder)
}
#[derive(Deserialize)]
struct Coordinate {
    x: f64,
    y: f64,
}

impl Target<'_> {
    pub(in crate::desktop) fn erase(
        &self,
        image: &Image,
        geometry: &mut source_resolution::Frame,
        crop: Option<crate::crop_geometry::Rect>,
        mut mapping: Option<crate::image_projection::Sampling>,
        cancel: &AtomicBool,
    ) -> Result<Image> {
        let Some(raw) = self.edit_state else {
            return Ok(image.clone());
        };
        let history: History<'_> = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
        let Some(raw) = history.strokes.filter(|raw| raw.get() != "null") else {
            return Ok(image.clone());
        };
        let history =
            strokes(&mut serde_json::Deserializer::from_str(raw.get())).map_err(|_| invalid())?;
        let mut total = 0usize;
        let mut output = image.clone();
        for raw in history {
            if cancel.load(Ordering::Acquire) {
                return Err(Error::new(
                    windows::Win32::Foundation::E_ABORT,
                    "Projection erase cancelled",
                ));
            }
            let stroke: Stroke<'_> = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
            total = total
                .checked_add(stroke.points.len())
                .filter(|count| *count <= 65_536)
                .ok_or_else(invalid)?;
            if stroke.points.is_empty() {
                continue;
            }
            // 只保留一笔的x/y；未知扩展、图片和其余历史始终保持借用。
            let _lease = crate::pixels::reserve(stroke.points.len() * 1024).ok_or_else(|| {
                Error::new(
                    windows::Win32::Foundation::E_OUTOFMEMORY,
                    "Projection erase parse budget exhausted",
                )
            })?;
            let mut points = Vec::new();
            points.try_reserve_exact(stroke.points.len()).map_err(|_| {
                Error::new(
                    windows::Win32::Foundation::E_OUTOFMEMORY,
                    "Cannot allocate projection erase points",
                )
            })?;
            for raw in stroke.points {
                if cancel.load(Ordering::Acquire) {
                    return Err(Error::new(
                        windows::Win32::Foundation::E_ABORT,
                        "Projection erase cancelled",
                    ));
                }
                let point: Coordinate = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
                points.push(serde_json::json!({"x":point.x,"y":point.y}));
            }
            let width = stroke
                .width
                .map(|raw| serde_json::from_str::<f64>(raw.get()))
                .transpose()
                .map_err(|_| invalid())?;
            let edit = serde_json::json!({"contentEraseStrokes":[{
                "width":width,"points":points}]});
            output = if let Some(mapping) = mapping.as_mut() {
                source_resolution::source_erase::prepare_mapped(
                    &output, geometry, crop, &edit, mapping, cancel,
                )?
            } else {
                source_resolution::source_erase::prepare(&output, geometry, crop, &edit, cancel)?
            };
            drop(edit);
        }
        Ok(output)
    }
}
