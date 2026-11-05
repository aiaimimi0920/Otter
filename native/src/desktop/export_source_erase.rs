//! 历史内容擦除只物化自然底图，笔迹使用当前可见框坐标，不影响后绘制标注。
use super::*;
#[path = "export_projection_mapped_erase.rs"]
mod mapped;
#[path = "export_source_erase_projection.rs"]
mod projection;
pub(in crate::desktop) use mapped::prepare_mapped;

pub(in crate::desktop) fn prepare(
    image: &Image,
    geometry: &mut Frame,
    crop: Option<Rect>,
    edit: &serde_json::Value,
    cancel: &AtomicBool,
) -> Result<Image> {
    let Some(strokes) = strokes(edit, cancel)? else {
        return Ok(image.clone());
    };
    let empty = matches!(geometry.sampling, Sampling::Empty);
    let sampled = projection::prepare(image, geometry, crop, cancel)?;
    let image = sampled.as_ref().unwrap_or(image);
    let layout = Layout::for_frame(image, geometry, crop)?;
    let frame = Rect {
        x: layout.base.left as f64 - layout.visible.x,
        y: layout.base.top as f64 - layout.visible.y,
        width: (layout.base.right - layout.base.left) as f64,
        height: (layout.base.bottom - layout.base.top) as f64,
    };
    apply(image, frame, empty, strokes, cancel)
}

fn strokes<'a>(
    edit: &'a serde_json::Value,
    cancel: &AtomicBool,
) -> Result<Option<&'a [serde_json::Value]>> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Source erase cancelled",
        ));
    }
    let strokes = match edit.get("contentEraseStrokes") {
        None | Some(serde_json::Value::Null) => return Ok(None),
        Some(value) => value
            .as_array()
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid erase strokes"))?,
    };
    if strokes.len() > 4096 {
        return Err(Error::new(E_INVALIDARG, "Too many erase strokes"));
    }
    Ok((!strokes
        .iter()
        .all(|stroke| stroke["points"].as_array().is_some_and(Vec::is_empty)))
    .then_some(strokes.as_slice()))
}

fn apply(
    image: &Image,
    frame: Rect,
    empty: bool,
    strokes: &[serde_json::Value],
    cancel: &AtomicBool,
) -> Result<Image> {
    let cancelled = || cancel.load(Ordering::Acquire);
    let mut output = image.clone();
    let mut total = 0usize;
    for stroke in strokes {
        if cancelled() {
            return Err(Error::new(
                windows::Win32::Foundation::E_ABORT,
                "Source erase cancelled",
            ));
        }
        let points = stroke["points"]
            .as_array()
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid erase points"))?;
        total = total
            .checked_add(points.len())
            .filter(|count| *count <= 65_536)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Too many erase points"))?;
        if points.is_empty() {
            continue;
        }
        let width = stroke["width"]
            .as_f64()
            .filter(|width| width.is_finite() && *width > 0.0 && *width <= 1_000_000.0)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid erase width"))?
            as f32;
        let bytes = points.len() * std::mem::size_of::<Point>();
        let mut lease = crate::pixels::reserve(bytes).ok_or_else(|| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Erase path budget exhausted",
            )
        })?;
        let mut path = Vec::new();
        path.try_reserve_exact(points.len()).map_err(|_| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Cannot allocate erase path",
            )
        })?;
        if !lease.grow_to(path.capacity() * std::mem::size_of::<Point>()) {
            return Err(Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Erase path capacity exceeds budget",
            ));
        }
        for point in points {
            let coordinate = |key| {
                point[key]
                    .as_f64()
                    .filter(|n| n.is_finite() && n.abs() <= 1_000_000.0)
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid erase coordinate"))
            };
            path.push(Point {
                x: coordinate("x")? as f32,
                y: coordinate("y")? as f32,
            });
        }
        if !empty {
            output.pixels = crate::erase::apply_history_in_frame(
                &output.pixels,
                (output.width, output.height),
                &path,
                width,
                frame,
                cancelled,
            )
            .map_err(|error| {
                Error::new(
                    match error {
                        crate::erase::EraseError::Cancelled => windows::Win32::Foundation::E_ABORT,
                        crate::erase::EraseError::BudgetExceeded => {
                            windows::Win32::Foundation::E_OUTOFMEMORY
                        }
                        _ => E_INVALIDARG,
                    },
                    "Cannot apply historical content erase",
                )
            })?;
        }
        // 路径先于租约释放；每笔输出替换上笔共享图，不保留整套历史副本。
        drop(path);
        drop(lease);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn cropped_points_map_to_natural_source_without_erasing_original() {
        let image = Image {
            width: 40,
            height: 20,
            pixels: [10, 20, 30, 255].repeat(800).into(),
        };
        let edit = json!({"contentEraseStrokes":[{"width":4,"points":[{"x":5,"y":5}]}]});
        let output = prepare(
            &image,
            &mut Size {
                width: 20.0,
                height: 10.0,
            }
            .into(),
            Some(Rect {
                x: 5.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
            &edit,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(output.pixels[(10 * 40 + 20) * 4 + 3], 0);
        assert_eq!(output.pixels[(10 * 40 + 10) * 4 + 3], 255);
        assert_eq!(image.pixels[(10 * 40 + 20) * 4 + 3], 255);
    }
    #[test]
    fn invalid_paths_and_cancellation_fail_without_mutating_source() {
        let image = Image {
            width: 4,
            height: 4,
            pixels: vec![255; 64].into(),
        };
        let logical = Size {
            width: 4.0,
            height: 4.0,
        };
        for edit in [
            json!({"contentEraseStrokes":false}),
            json!({"contentEraseStrokes":[{"width":4,"points":[{"x":"bad","y":0}]}]}),
            json!({"contentEraseStrokes":[{"width":0,"points":[{"x":0,"y":0}]}]}),
        ] {
            assert!(prepare(
                &image,
                &mut logical.into(),
                None,
                &edit,
                &AtomicBool::new(false)
            )
            .is_err());
        }
        assert!(prepare(
            &image,
            &mut logical.into(),
            None,
            &json!({}),
            &AtomicBool::new(true)
        )
        .is_err());
        assert!(image.pixels.iter().all(|v| *v == 255));
    }
}
