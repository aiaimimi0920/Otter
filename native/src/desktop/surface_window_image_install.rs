//! 授权窗口已安装资源；消费时同 Grant 转交，不持有网络任务槽，按当前快照投影。
use super::*;
use crate::{
    surface_image_paint::Bitmap,
    surface_presentation::Images,
    surface_resource_runtime::request::{Key, Request},
    surface_scene::{Kind, Node, Scene},
};
use std::{collections::BTreeMap, rc::Rc};
use windows::Win32::Foundation::E_INVALIDARG;
pub(in crate::desktop::surface_authorized_window) struct Installed {
    pub request: Request,
    bitmap: Rc<Bitmap>,
}
pub(in crate::desktop::surface_authorized_window) type InstalledImages =
    BTreeMap<Key, Rc<Installed>>;
fn projection(current: &MountedSurface, installed: &InstalledImages) -> Result<(Scene, Images)> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        scene: Box<serde_json::value::RawValue>,
    }
    let envelope: Envelope = serde_json::from_str(current.snapshot.get())
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid image scene envelope"))?;
    let scene = Scene::parse(envelope.scene.get().as_bytes())
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid image scene"))?;
    fn visit(node: &Node, installed: &InstalledImages, images: &mut Images) {
        if !node.visible() {
            return;
        }
        if node.kind() == Kind::Image {
            if let Some(key) = crate::surface_resource_runtime::request::inline::key(node) {
                if let Some(entry) = installed.get(&key) {
                    images.insert(node.id().to_owned(), entry.bitmap.clone());
                }
            }
            return;
        }
        for child in node.visible_children() {
            visit(child, installed, images);
        }
    }
    let mut images = Images::new();
    visit(scene.root(), installed, &mut images);
    Ok((
        scene.with_image_availability(|id| images.contains_key(id)),
        images,
    ))
}
impl AuthorizedWindow {
    #[cfg(test)]
    pub(crate) fn image_placeholder_for_test(&self, id: &str) -> Option<bool> {
        self.window.image_placeholder_for_test(id)
    }
    /// 只接纳当前窗口/场景及同 Grant 的 Ready；错误消费为终态，不重新排入 Ready 队列。
    pub fn install_resource(
        &mut self,
        ready: Ready,
        now: u64,
    ) -> std::result::Result<(), ImageError> {
        let current = self.resource_mount(now)?;
        let (request, pixels, logical_size) = ready
            .transfer_image(&self.lease, &current, now)
            .map_err(ImageError::Ready)?;
        let bitmap = Bitmap::with_logical_size(pixels, logical_size).map_err(ImageError::Window)?;
        let mut next = self.images.borrow().clone();
        if !next.contains_key(&request.key()) && next.len() >= 64 {
            return Err(ImageError::Resource(ResourceError::Limit));
        }
        next.insert(request.key(), Rc::new(Installed { request, bitmap }));
        self.commit_images(&current, next)
            .map_err(ImageError::Window)?;
        // HWND 可能同步重入；同一 Grant 的窗口租约继续负责撤权，不沿用消费前的检查。
        self.refresh(now).map_err(ImageError::Window)?;
        if !self.is_alive() || self.stopped.is_some() {
            return Err(ImageError::Inactive);
        }
        Ok(())
    }
    fn commit_images(&mut self, current: &MountedSurface, images: InstalledImages) -> Result<()> {
        let (scene, projected) = projection(current, &images)?;
        if let Err(error) = self.window.refresh_images(scene, projected) {
            // 同场景图片更新也可能在 HWND 提交后失败，不能继续暴露半提交状态。
            let _ = self.close();
            return Err(error);
        }
        if !self.is_alive() {
            self.images.get_mut().clear();
            self.resources.get_mut().take();
            return Err(Error::new(
                E_FAIL,
                "Surface closed during image installation",
            ));
        }
        *self.images.get_mut() = images;
        Ok(())
    }
    pub(in crate::desktop::surface_authorized_window) fn refresh_images(
        &mut self,
        now: u64,
    ) -> Result<()> {
        if self.images.get_mut().is_empty() {
            return Ok(());
        }
        // 不调用 resource_mount/refresh，避免递归；RefCell 借用在 HWND 调用前结束。
        let current = self
            .resources
            .borrow()
            .as_ref()
            .and_then(|snapshot| snapshot.mount(self.binding(), self.snapshot_revision).ok());
        let Some(current) = current else {
            return self.close();
        };
        let mut next = self.images.borrow().clone();
        // 一次 refresh 只散列一次原始快照，不按每张内联图重新解析/复制整棵 Scene。
        let digest = next
            .keys()
            .any(|key| matches!(key, Key::Node(_)))
            .then(|| crate::surface_resource_runtime::request::inline::digest(&current));
        next.retain(|_, image| {
            image
                .request
                .validate_with_digest(&self.lease, &current, now, digest.as_ref())
                .is_ok()
        });
        if next.len() != self.images.borrow().len() {
            self.commit_images(&current, next)?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn installed_images_for_test(&self) -> usize {
        self.images.borrow().len()
    }
    #[cfg(test)]
    pub(crate) fn installed_image_size_for_test(&self, node: &str) -> Option<[f32; 2]> {
        self.images
            .borrow()
            .get(&Key::Node(node.to_owned()))
            .map(|image| image.bitmap.natural_size())
    }
    #[cfg(test)]
    pub(crate) fn uploaded_images_for_test(&self) -> usize {
        self.images
            .borrow()
            .values()
            .filter(|image| image.bitmap.uploaded_for_test())
            .count()
    }
}
