//! 顺序键盘导航先显露目标，再交付可见焦点；不创建动作或改变提交权限。
use super::*;
impl Presentation {
    pub(crate) fn can_focus(&self, id: &str) -> bool {
        self.navigation.iter().any(|candidate| candidate == id)
    }
    pub(crate) fn reveal_focus(&mut self, id: &str) -> Result<bool> {
        if !self.can_focus(id) {
            return Ok(false);
        }
        let mut scroll = self.scroll.clone();
        if scroll.reveal(&self.geometry, id) {
            let frame = self
                .cache
                .project_with_images(&self.scene, &mut scroll, &self.images)?;
            let focusable = crate::surface_focus::buttons(&self.scene, &frame.geometry);
            // 不可显露候选不能先滚动视图；准备成功且可见才一起提交。
            if focusable.iter().any(|candidate| candidate == id) {
                self.install_frame(frame, scroll);
                return Ok(true);
            }
            Ok(false)
        } else {
            Ok(self.focusable(id))
        }
    }
    pub(crate) fn reveal_next_focus(
        &mut self,
        current: Option<&str>,
        reverse: bool,
    ) -> Result<Option<String>> {
        // 导航集合包含屏外控件，动作可用集合仍只包含当前可见控件。
        let mut candidate = current.map(str::to_owned);
        for _ in 0..self.navigation.len() {
            let Some(next) =
                crate::surface_focus::next(&self.navigation, candidate.as_deref(), reverse)
                    .map(str::to_owned)
            else {
                break;
            };
            if self.reveal_focus(&next)? {
                return Ok(Some(next));
            }
            // 静态零面积祖先等不可显露目标不能阻断整个 Tab 环。
            candidate = Some(next);
        }
        Ok(None)
    }
}
