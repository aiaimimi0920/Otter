//! 外部显示快照复用窗口绘制目标；不交换文档编辑器，不带入目标的草稿和选择手柄。
use super::*;

impl Canvas {
    pub(super) unsafe fn draw_display(
        &mut self,
        frame: &crate::desktop::display_frame::Frame,
        size: D2D_SIZE_U,
        dpi: u32,
    ) -> Result<()> {
        let empty = Editor::default();
        let editor = frame.editor.as_deref().unwrap_or(&empty);
        let image = &frame.image;
        let resources = self.target.as_mut().expect("paint owns target creation");
        resources.surface.resize(size)?;
        let target = resources.surface.resource_target();
        resources.raster.sync_sized(
            target,
            Some(image),
            editor
                .layers()
                .and_then(|layers| layers.annotations.as_ref()),
            editor
                .layers()
                .map_or((0, 0), |layers| layers.annotation_dimensions()),
        )?;
        let origin = self.viewport.origin;
        let scale = self.viewport.scale;
        let rect = editor.raster_base_projection().map_or(
            D2D_RECT_F {
                left: origin.x,
                top: origin.y,
                right: origin.x + image.width as f32 * scale,
                bottom: origin.y + image.height as f32 * scale,
            },
            |mapping| {
                let r = mapping.target;
                D2D_RECT_F {
                    left: origin.x + r.x as f32 * scale,
                    top: origin.y + r.y as f32 * scale,
                    right: origin.x + (r.x + r.width) as f32 * scale,
                    bottom: origin.y + (r.y + r.height) as f32 * scale,
                }
            },
        );
        let clip = editor.annotation_frame().map_or(rect, |r| D2D_RECT_F {
            left: origin.x + r.x as f32 * scale,
            top: origin.y + r.y as f32 * scale,
            right: origin.x + (r.x + r.width) as f32 * scale,
            bottom: origin.y + (r.y + r.height) as f32 * scale,
        });
        target.SetDpi(dpi as f32, dpi as f32);
        resources.surface.draw(|target| {
            let background = if self.pinned {
                D2D1_COLOR_F::default()
            } else {
                D2D1_COLOR_F {
                    r: 0.07,
                    g: 0.08,
                    b: 0.08,
                    a: 1.0,
                }
            };
            target.Clear(Some(&background));
            let _clipped = super::super::render_frame::Clip::push(target, &clip);
            // 走无擦除的公共路径同时退休旧擦除几何缓存。
            resources
                .raster
                .draw_erasing_projected(
                    target,
                    &rect,
                    &clip,
                    None,
                    self.viewport,
                    editor.raster_base_projection(),
                )
                .and_then(|_| {
                    crate::desktop::annotations::draw_preview(
                        target,
                        &resources.brush,
                        editor,
                        self.viewport,
                        None,
                        None,
                        &mut resources.effects,
                    )
                })
        })?;
        self.paints = self.paints.saturating_add(1);
        Ok(())
    }
}
