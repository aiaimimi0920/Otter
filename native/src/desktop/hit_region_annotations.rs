//! 显示编辑快照的矢量标注命中区域；不读取目标 Canvas 的编辑状态。
use super::*;

pub(super) unsafe fn add_annotations(
    region: &OwnedRegion,
    editor: &crate::editor::Editor,
    scale: f32,
    dpi: u32,
) -> Result<()> {
    let scale = scale * dpi as f32 / 96.0;
    add_transformed(
        region,
        editor,
        XFORM {
            eM11: scale,
            eM22: scale,
            ..Default::default()
        },
    )
}

pub(super) unsafe fn add_transformed(
    region: &OwnedRegion,
    editor: &crate::editor::Editor,
    transform: XFORM,
) -> Result<()> {
    for stroke in editor.elements() {
        if !stroke.visible_paint() {
            continue;
        }
        let mut bounds = (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        );
        let arrow = stroke
            .has_straight_arrow()
            .then(|| crate::arrow_geometry::build_path(&stroke.points, stroke.width))
            .flatten();
        if (stroke.has_straight_arrow() && arrow.is_none())
            || (stroke.tool == crate::editor::Tool::Line
                && stroke
                    .points
                    .first()
                    .is_some_and(|p| stroke.points.iter().all(|q| p == q)))
        {
            continue;
        }
        let mut include = |point: crate::viewport::Point| {
            bounds.0 = bounds.0.min(point.x);
            bounds.1 = bounds.1.min(point.y);
            bounds.2 = bounds.2.max(point.x);
            bounds.3 = bounds.3.max(point.y);
        };
        if let Some(arrow) = arrow {
            for point in arrow.shaft().chain(arrow.arrow.head) {
                include(point);
            }
        } else {
            for &point in &stroke.points {
                include(point);
            }
        }
        if !bounds.0.is_finite() {
            continue;
        }
        let pad = if stroke.tool == crate::editor::Tool::Bezier && stroke.arrow {
            (stroke.width * 6.0).max(24.0) + (stroke.width * 5.0).max(16.0) / 2.0 + 1.0
        } else if matches!(
            stroke.tool,
            crate::editor::Tool::Triangle | crate::editor::Tool::Polygon
        ) {
            stroke.width * 5.0 + 1.0
        } else {
            stroke.width / 2.0 + 1.0
        };
        let rotated = crate::rotation::bounds(
            (
                bounds.0 - pad,
                bounds.1 - pad,
                bounds.2 + pad,
                bounds.3 + pad,
            ),
            crate::rotation::center(&stroke.points),
            stroke.rotation,
        );
        bounds = (
            rotated.0 + pad,
            rotated.1 + pad,
            rotated.2 - pad,
            rotated.3 - pad,
        );
        let annotation = OwnedRegion::checked(CreateRectRgn(
            ((bounds.0 - pad) * transform.eM11 + transform.eDx).floor() as i32,
            ((bounds.1 - pad) * transform.eM22 + transform.eDy).floor() as i32,
            ((bounds.2 + pad) * transform.eM11 + transform.eDx).ceil() as i32,
            ((bounds.3 + pad) * transform.eM22 + transform.eDy).ceil() as i32,
        ))?;
        if CombineRgn(Some(region.0), Some(region.0), Some(annotation.0), RGN_OR).0 == ERROR {
            return Err(Error::from_win32());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor(points: serde_json::Value) -> crate::editor::Editor {
        let raw =
            serde_json::value::to_raw_value(&serde_json::json!({"serialCounter":0,"elements":[{
            "id":"window-arrow","type":"arrow","zIndex":0,"points":points,
            "style":{"width":4,"color":"#ff0000"}}]}))
            .unwrap();
        crate::session_vectors::restore(Some(&raw)).unwrap().editor
    }
    #[test]
    fn bent_arrow_window_region_includes_actual_final_segment_head() {
        let editor = editor(serde_json::json!([{"x":8,"y":8},{"x":8,"y":48},{"x":56,"y":48}]));
        unsafe {
            let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
            add_annotations(&region, &editor, 1.0, 96).unwrap();
            assert!(PtInRegion(region.0, 34, 58).as_bool());
            assert!(!PtInRegion(region.0, 34, 62).as_bool());
        }
    }
    #[test]
    fn coincident_arrow_does_not_add_invisible_window_region() {
        let editor = editor(serde_json::json!([{"x":24,"y":24},{"x":24,"y":24}]));
        unsafe {
            let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
            add_annotations(&region, &editor, 1.0, 96).unwrap();
            assert!(!PtInRegion(region.0, 24, 24).as_bool());
        }
    }
}
