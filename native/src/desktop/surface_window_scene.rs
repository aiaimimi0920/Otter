//! HWND 场景提交接缝；替换时撤销旧无障碍身份，复用宿主及重入门禁。
use super::*;
impl SurfaceWindow {
    pub(in crate::desktop) fn viewport_epoch(&self) -> u64 {
        self.state.viewport_epoch.get()
    }
    pub(in crate::desktop) fn viewport(&self) -> Result<crate::surface_viewport::Viewport> {
        if !self.is_alive() || self.viewport_epoch() == u64::MAX {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        crate::surface_viewport::Viewport::from_window(self.hwnd)
    }
    #[cfg(test)]
    pub(crate) fn image_placeholder_for_test(&self, id: &str) -> Option<bool> {
        fn find(node: &crate::surface_scene::Node, id: &str) -> Option<bool> {
            if !node.visible() {
                return None;
            }
            if node.id() == id {
                return Some(node.image_placeholder());
            }
            node.visible_children().find_map(|child| find(child, id))
        }
        self.state
            .host
            .borrow()
            .as_ref()
            .and_then(|host| find(host.presentation().scene().root(), id))
    }
    pub fn replace(&self, scene: Scene) -> Result<()> {
        self.replace_with_images(scene, crate::surface_presentation::Images::new())
    }
    /// 同节点 ID 不继承旧图；调用方负责资源版本、租约及可重入调用后的授权复核。
    pub fn replace_with_images(
        &self,
        scene: Scene,
        images: crate::surface_presentation::Images,
    ) -> Result<()> {
        self.replace_image_scene(scene, images, false, false)
    }
    pub(in crate::desktop) fn refresh_images(
        &self,
        scene: Scene,
        images: crate::surface_presentation::Images,
    ) -> Result<()> {
        self.replace_image_scene(scene, images, true, false)
    }
    pub(crate) fn replace_javascript_frame(
        &self,
        scene: Scene,
        images: crate::surface_presentation::Images,
    ) -> Result<()> {
        self.replace_image_scene(scene, images, false, true)
    }
    fn replace_image_scene(
        &self,
        scene: Scene,
        images: crate::surface_presentation::Images,
        same_scene: bool,
        javascript: bool,
    ) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        let accessibility = crate::surface_accessibility::Owner::new(&scene);
        self.state.notifications.borrow_mut().clear();
        // 替换期间先撤销旧描述；失败时保持不可用，不把部分替换当作旧合同。
        self.state.accessibility.borrow_mut().take();
        structure_notifications::schedule(&self.state);
        if !self.state.run(|host| {
            if same_scene {
                host.refresh_images(scene, images)
            } else {
                host.replace_with_images(scene, images)
            }
        })? {
            return Err(Error::new(E_FAIL, "Surface window busy"));
        }
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        *self.state.accessibility.borrow_mut() = Some(accessibility);
        if let Some(root) = self.state.uia.borrow().as_ref() {
            root.set_scene(
                self.state
                    .accessibility
                    .borrow()
                    .as_ref()
                    .and_then(|owner| owner.root()),
            )?;
        }
        unsafe {
            keyboard::reconcile_scene(self.hwnd, &self.state, javascript);
        }
        self.invalidate()
    }
}
