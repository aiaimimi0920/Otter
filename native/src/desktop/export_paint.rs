//! 单一导出绘制事务；窗口逻辑样式与文档坐标分别变换，圆角包住完整内容栈。
use super::*;
pub(super) struct Paint<'a> {
    pub target: &'a ID2D1RenderTarget,
    pub factory: &'a ID2D1Factory,
    pub bitmap: &'a ID2D1Bitmap,
    pub annotations: Option<&'a ID2D1Bitmap>,
    pub brush: &'a ID2D1SolidColorBrush,
    pub editor: &'a Editor,
}
impl Paint<'_> {
    pub(super) unsafe fn draw(
        self,
        opacity: f32,
        layout: Option<&source_resolution::Layout>,
        decoration: Option<&style::Snapshot>,
        output: (u32, u32),
        reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<()> {
        let target = self.target;
        super::super::render_frame::draw(target, || {
            target.Clear(Some(&D2D1_COLOR_F::default()));
            let crop = layout
                .map(|layout| layout.visible)
                .or_else(|| self.editor.annotation_frame());
            let mut document = Default::default();
            target.GetTransform(&mut document);
            if let Some(crop) = crop {
                document.M11 = output.0 as f32 / crop.width as f32;
                document.M22 = output.1 as f32 / crop.height as f32;
                document.M31 = -(crop.x as f32) * document.M11;
                document.M32 = -(crop.y as f32) * document.M22;
            }
            let _restore = super::super::render_frame::save_transform(target);
            // 先在 unit 坐标推 mask，再切回文档坐标；非等比输出也保持两轴源密度。
            let rounded = if let Some(style) = decoration {
                style.transform(target, output);
                style.clip(target, self.factory, output, &reserve)?
            } else {
                target.SetTransform(&document);
                source_resolution::frame_clip::push(target, self.factory, layout, &reserve)?
            };
            target.SetTransform(&document);
            source_resolution::draw(target, self.bitmap, layout, opacity);
            if let Some(style) = decoration {
                style.transform(target, output);
                style.border(target)?;
                target.SetTransform(&document);
            } else if let Some(border) = layout.and_then(|layout| layout.border.as_ref()) {
                border.draw(target, layout.unwrap().visible)?;
            }
            if let Some(bitmap) = self.annotations {
                let destination = crop
                    .map(|r| D2D_RECT_F {
                        left: r.x as f32,
                        top: r.y as f32,
                        right: (r.x + r.width) as f32,
                        bottom: (r.y + r.height) as f32,
                    })
                    .unwrap_or(D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: output.0 as f32,
                        bottom: output.1 as f32,
                    });
                target.DrawBitmap(
                    bitmap,
                    Some(&destination),
                    1.0,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    None,
                );
            }
            annotations::draw_committed(
                target,
                self.brush,
                self.editor,
                Viewport {
                    origin: Point::default(),
                    scale: 1.0,
                },
                &mut super::super::effect_cache::Cache::default(),
            )?;
            drop(rounded);
            Ok(())
        })
    }
}
