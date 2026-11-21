//! 同一编辑几何驱动 Live 覆盖层和源输入，保持裁剪、源尺寸变化与镜像一致。
use crate::{crop_geometry::Rect, editor::Editor, image_projection::Projection};

pub(super) fn geometry(editor: &Editor) -> (Rect, Projection, [f64; 2]) {
    let (width, height) = editor.layers().expect("Live editor raster").size;
    let full = Rect {
        x: 0.0,
        y: 0.0,
        width: width as f64,
        height: height as f64,
    };
    let bounds = editor.annotation_frame().unwrap_or(full);
    let mapping = editor.raster_base_projection().unwrap_or(Projection {
        source: full,
        target: full,
    });
    (bounds, mapping, [full.width, full.height])
}
pub(super) fn viewport(
    editor: &Editor,
    destination: [u32; 2],
) -> Option<crate::live_viewport::Viewport> {
    let (bounds, mapping, raster) = geometry(editor);
    let source = mapping.source;
    let target = mapping.target;
    let region = [
        (source.x + (bounds.x - target.x) * source.width / target.width) / raster[0],
        (source.y + (bounds.y - target.y) * source.height / target.height) / raster[1],
        bounds.width * source.width / target.width / raster[0],
        bounds.height * source.height / target.height / raster[1],
    ];
    let mirror = editor.mirror_state();
    crate::live_viewport::Viewport::fit_content(
        [raster[0] as u32, raster[1] as u32],
        destination,
        [bounds.width, bounds.height],
    )
    .map(|view| view.with_region(region, [mirror.x, mirror.y]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_pin_cropped_mirrored_input_matches_display_and_rejects_letterbox() {
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (200, 100),
            base: vec![255; 200 * 100 * 4].into(),
            annotation_size: None,
            annotations: None,
        })
        .unwrap();
        let plan = editor
            .prepare_flip(crate::raster_flip::Axis::Horizontal, |_| unreachable!())
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert!(editor
            .crop_to(Rect {
                x: 20.0,
                y: 10.0,
                width: 100.0,
                height: 50.0
            })
            .unwrap());
        let view = viewport(&editor, [400, 400]).unwrap();
        assert_eq!(view.destination(), [0.0, 100.0, 400.0, 300.0]);
        assert_eq!(view.normalized([0.0, 100.0], false), Some([0.9, 0.1]));
        assert_eq!(view.normalized([200.0, 200.0], false), Some([0.65, 0.35]));
        assert!(view.normalized([200.0, 99.0], false).is_none());
    }
}
