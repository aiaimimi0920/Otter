//! 滚动条是宿主覆盖层：绘制与命中共用矩形，先于内容鼠标动作消费。
use super::*;
use crate::surface_scroll::Bar;
use windows::Win32::Graphics::Direct2D::{Common::*, *};
impl Presentation {
    pub(crate) fn pan(&mut self, anchor: &str, delta: [f32; 2]) -> Result<bool> {
        if delta.iter().any(|v| !v.is_finite()) {
            return Ok(false);
        }
        let mut scroll = self.scroll.clone();
        let mut changed = false;
        for (axis, distance) in delta.into_iter().enumerate() {
            changed |= scroll.wheel(&self.geometry, anchor, axis, distance, false);
        }
        if !changed {
            return Ok(false);
        }
        // 两轴只投影一次，不在中间帧重命中或重新测量文字。
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        self.install_frame(frame, scroll);
        Ok(true)
    }
    pub(crate) fn scroll_key(
        &mut self,
        anchor: Option<&str>,
        command: crate::surface_scroll::Command,
    ) -> Result<bool> {
        let anchor = anchor.unwrap_or(self.scene.root().id());
        let mut scroll = self.scroll.clone();
        if !scroll.keyboard(&self.geometry, anchor, command) {
            return Ok(false);
        }
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        self.install_frame(frame, scroll);
        Ok(true)
    }
    pub(crate) fn scrollbars(&self) -> Vec<Bar> {
        self.scroll.bars(&self.geometry)
    }
    pub(crate) fn scrollbar_at(&self, point: [f32; 2]) -> Option<Bar> {
        self.scrollbars()
            .into_iter()
            .rev()
            .find(|bar| bar.hit(point))
    }
    pub(crate) fn set_scrollbar(&mut self, id: &str, axis: usize, value: f32) -> Result<bool> {
        let mut scroll = self.scroll.clone();
        if !scroll.set_bar(id, axis, value) {
            return Ok(false);
        }
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        self.install_frame(frame, scroll);
        Ok(true)
    }
    pub(super) fn draw_scrollbars(&self, target: &ID2D1RenderTarget, brush: &ID2D1SolidColorBrush) {
        let rect = |r: crate::surface_clip::Rect| D2D_RECT_F {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        };
        unsafe {
            let previous = brush.GetColor();
            for bar in self.scrollbars() {
                target.PushAxisAlignedClip(&rect(bar.clip), D2D1_ANTIALIAS_MODE_ALIASED);
                // Neuro control 与 textMuted 中性色；覆盖层不抢占信号色语义。
                brush.SetColor(&D2D1_COLOR_F {
                    r: 17. / 255.,
                    g: 23. / 255.,
                    b: 32. / 255.,
                    a: 1.,
                });
                target.FillRectangle(&rect(bar.track), brush);
                brush.SetColor(&D2D1_COLOR_F {
                    r: 146. / 255.,
                    g: 154. / 255.,
                    b: 159. / 255.,
                    a: 1.,
                });
                target.FillRectangle(&rect(bar.thumb), brush);
                target.PopAxisAlignedClip();
            }
            brush.SetColor(&previous);
        }
    }
}
