//! 外部显示快照复用窗口绘制目标；不交换文档编辑器，不带入目标的草稿和选择手柄。
use super::*;

impl Canvas {
    pub(super) unsafe fn draw_display(
        &mut self,
        frame: &crate::desktop::display_frame::Frame,
        size: D2D_SIZE_U,
        dpi: u32,
    ) -> Result<()> {
        let editor = frame.editor.as_deref();
        let image = &frame.image;
        let resources = self.target.as_mut().expect("paint owns target creation");
        resources.surface.resize(size)?;
        let target = resources.surface.resource_target();
        resources.raster.sync_frame(target, Some(image), editor)?;
        let raster::Geometry {
            base,
            clip,
            projection,
        } = raster::Geometry::new(image, editor, self.viewport);
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
            let frame_style =
                crate::desktop::frame_style_draw::Frame::new(self.frame_style, size, dpi);
            let _rounded = frame_style.clip(target, &self.factory)?;
            let clipped = super::super::render_frame::Clip::push(target, &clip);
            // 走无擦除的公共路径同时退休旧擦除几何缓存。
            resources.raster.draw_interleaved(
                target,
                &base,
                &clip,
                None,
                (self.viewport, projection),
                || frame_style.border_over_clip(target, &clip),
            )?;
            if let Some(editor) = editor {
                crate::desktop::annotations::draw_preview(
                    target,
                    &resources.brush,
                    editor,
                    self.viewport,
                    None,
                    None,
                    &mut resources.effects,
                )?;
            } else {
                resources.effects.retain_tools(false, false);
                resources.effects.annotations = Default::default();
            }
            drop(clipped);
            if let Some(views) = &self.extension_views {
                views.draw(
                    &self.editor,
                    Some(frame),
                    target,
                    &resources.brush,
                    (size, dpi),
                )?;
            }
            Ok(())
        })?;
        self.paints = self.paints.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_without_editor_releases_annotations_without_replacing_target() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let image = Image {
            width: 32,
            height: 32,
            pixels: vec![255; 32 * 32 * 4].into(),
        };
        let mut canvas = Canvas::new(Some(image.clone())).unwrap();
        canvas.software_renderer = true;
        canvas
            .editor
            .append_restored(
                Tool::Brush,
                vec![Point { x: 2., y: 2. }, Point { x: 20., y: 18. }],
                0.,
                Vec::new(),
            )
            .unwrap();
        canvas
            .editor
            .insert_text(
                Point { x: 2., y: 2. },
                Point { x: 30., y: 30. },
                crate::text_annotation::TextAnnotation::new("缓存", "Segoe UI", 16., false)
                    .unwrap(),
            )
            .unwrap();
        let source = crate::workspace_id::UnitId::parse("cache-release").unwrap();
        let annotated = crate::desktop::display_frame::Frame::capture(
            source.clone(),
            image.clone(),
            Some(&canvas.editor),
        );
        canvas
            .paint_display(HWND::default(), 32, 32, 96, Some(&annotated))
            .unwrap();
        let target = canvas.target.as_ref().unwrap();
        let identity = target.identity.clone();
        let counts = target.effects.annotations.entry_counts();
        assert!(counts[0] > 0 && counts[1] > 0);
        let plain = crate::desktop::display_frame::Frame::capture(source, image, None);
        canvas
            .paint_display(HWND::default(), 32, 32, 96, Some(&plain))
            .unwrap();
        let target = canvas.target.as_ref().unwrap();
        assert!(std::rc::Rc::ptr_eq(&identity, &target.identity));
        assert_eq!(target.effects.annotations.entry_counts(), [0; 3]);
    }
}
