//! Vector-annotation hit regions for display editing snapshots; do not read target Canvas edit state.
use super::*;

pub(super) unsafe fn add_annotations(
    region: &OwnedRegion,
    editor: &crate::editor::Editor,
    image_size: (u32, u32),
    scale: f32,
    dpi: u32,
) -> Result<()> {
    let scale = scale * dpi as f32 / 96.0;
    add_transformed(
        region,
        editor,
        crate::crop_geometry::Size {
            width: image_size.0 as f64,
            height: image_size.1 as f64,
        },
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
    fallback: crate::crop_geometry::Size,
    transform: XFORM,
) -> Result<()> {
    if editor.has_spotlight() {
        // Art overlays intentionally have no raster/logical frame; their supplied
        // display bounds still own the visible dimming mask.
        let frame = editor
            .annotation_frame()
            .or_else(|| editor.visible_frame())
            .unwrap_or(crate::crop_geometry::Rect {
                x: 0.,
                y: 0.,
                width: fallback.width,
                height: fallback.height,
            });
        let dimmed = OwnedRegion::checked(CreateRectRgn(
            (frame.x as f32 * transform.eM11 + transform.eDx).floor() as i32,
            (frame.y as f32 * transform.eM22 + transform.eDy).floor() as i32,
            ((frame.x + frame.width) as f32 * transform.eM11 + transform.eDx).ceil() as i32,
            ((frame.y + frame.height) as f32 * transform.eM22 + transform.eDy).ceil() as i32,
        ))?;
        if CombineRgn(Some(region.0), Some(region.0), Some(dimmed.0), RGN_OR).0 == ERROR {
            return Err(Error::from_win32());
        }
    }
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
        let serial_line = stroke
            .serial_line()
            .filter(|_| stroke.paint_alpha.stroke() > 0.);
        if let Some(arrow) = arrow {
            for point in arrow.shaft().chain(arrow.arrow.head) {
                include(point);
            }
        } else {
            for &point in &stroke.points {
                include(point);
            }
            if let Some(line) = serial_line {
                for point in line {
                    include(point);
                }
            }
        }
        if !bounds.0.is_finite() {
            continue;
        }
        let pad = if stroke.tool == crate::editor::Tool::Bezier && stroke.arrow {
            (stroke.width * 6.0).max(24.0) + (stroke.width * 5.0).max(16.0) / 2.0 + 1.0
        } else if serial_line.is_some() {
            (stroke.width * 5.0).max(16.0) / 2.0 + 1.0
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
            add_annotations(&region, &editor, (64, 64), 1.0, 96).unwrap();
            assert!(PtInRegion(region.0, 34, 58).as_bool());
            assert!(!PtInRegion(region.0, 34, 62).as_bool());
        }
    }
    #[test]
    fn coincident_arrow_does_not_add_invisible_window_region() {
        let editor = editor(serde_json::json!([{"x":24,"y":24},{"x":24,"y":24}]));
        unsafe {
            let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
            add_annotations(&region, &editor, (64, 64), 1.0, 96).unwrap();
            assert!(!PtInRegion(region.0, 24, 24).as_bool());
        }
    }

    #[test]
    fn transparent_serial_leader_does_not_intercept_input_outside_visible_badge() {
        let mut editor = crate::editor::Editor::default();
        editor
            .begin_serial(
                crate::viewport::Point { x: 20., y: 20. },
                10.,
                "Arial",
                None,
            )
            .unwrap();
        editor
            .extend(crate::viewport::Point { x: 90., y: 20. })
            .unwrap();
        assert!(editor.commit());
        let style = editor.elements()[0]
            .style()
            .with_fill(Some(crate::stroke_style::Color(255, 0, 0)))
            .with_paint_alpha(crate::stroke_style::PaintAlpha::new(0., 1.).unwrap());
        assert!(editor.restyle(0, style));
        unsafe {
            let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
            add_annotations(&region, &editor, (100, 40), 1., 96).unwrap();
            assert!(PtInRegion(region.0, 20, 20).as_bool());
            assert!(!PtInRegion(region.0, 85, 20).as_bool());
        }
    }

    #[test]
    fn overlay_spotlight_keeps_dimmed_transparent_pixels_interactive() {
        let image = crate::desktop::image::Image {
            width: 32,
            height: 32,
            pixels: vec![0; 32 * 32 * 4].into(),
        };
        let mut editor = crate::desktop::Canvas::new(Some(image.clone()))
            .unwrap()
            .editor;
        editor
            .begin(Tool::SpotRect, crate::viewport::Point { x: 8., y: 8. })
            .unwrap();
        editor
            .extend(crate::viewport::Point { x: 16., y: 16. })
            .unwrap();
        assert!(editor.commit());
        let id = crate::workspace_id::UnitId::parse("spotlight-hit").unwrap();
        let frame =
            crate::desktop::display_frame::Frame::capture_overlay(id.clone(), id, image, &editor)
                .unwrap();
        let editor = frame.editor.as_deref().unwrap();
        assert!(editor.annotation_frame().is_none());
        unsafe {
            let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
            add_annotations(
                &region,
                editor,
                (frame.image.width, frame.image.height),
                1.,
                96,
            )
            .unwrap();
            assert!(PtInRegion(region.0, 28, 28).as_bool());
        }
    }
}
