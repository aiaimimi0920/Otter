//! 嵌入控件的绘制缓存准备/瞬态预览；不改变scene或普通Surface子HWND的交互所有权。
use super::{Command, PaintFrame};
use crate::surface_embedded_control::Paint;
impl PaintFrame {
    pub(crate) fn preview_text(
        &mut self,
        scene: &crate::surface_scene::Scene,
        font: Option<&crate::surface_presentation::ControlFont>,
        id: &str,
        value: Option<&str>,
    ) -> windows::core::Result<bool> {
        let marker = self.commands.iter().find_map(|command| match command {
            Command::Control(marker) if marker.id == id => Some(marker),
            _ => None,
        });
        let Some((marker, node)) = marker.zip(scene.root().visible_node(id)) else {
            return Ok(false);
        };
        self.controls
            .insert(id.into(), Paint::text_preview(node, marker, font, value)?);
        Ok(true)
    }
    pub(crate) fn preview_slider(&mut self, id: &str, value: f32) -> bool {
        if let Some(Paint::Slider { fraction, .. }) = self.controls.get_mut(id) {
            *fraction = value;
            true
        } else {
            false
        }
    }
    #[cfg(test)]
    pub(crate) fn embedded_control(&self, id: &str) -> Option<&Paint> {
        self.controls.get(id)
    }
    pub(crate) fn embed_controls<'a>(
        &mut self,
        scene: &crate::surface_scene::Scene,
        font: impl Fn(&str) -> Option<&'a crate::surface_presentation::ControlFont>,
    ) -> windows::core::Result<()> {
        self.controls = crate::surface_embedded_control::prepare(
            scene,
            self.commands.iter().filter_map(|c| {
                if let Command::Control(m) = c {
                    Some(m)
                } else {
                    None
                }
            }),
            font,
        )?;
        Ok(())
    }
}
