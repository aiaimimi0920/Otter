//! 内联图片只绑定原始场景，不授予 URL、文件或独立远端资源访问权限。
use super::{Key, ResourceError};
use crate::{
    live_relay_http::MountedSurface,
    surface_live_binding::Binding,
    surface_scene::{Kind, Node, Scene},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub struct Inline {
    binding: Binding,
    revision: u64,
    node: String,
    source: Arc<str>,
    mime: String,
    snapshot_digest: [u8; 32],
}
/// 与旧 imageSource 一致：非空 resourceId 优先，失败也不能退回 src。
pub(crate) fn key(node: &Node) -> Option<Key> {
    if node.kind() != Kind::Image {
        return None;
    }
    if let Some(id) = node
        .props()
        .get("resourceId")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
    {
        return Some(Key::Resource(id.to_owned()));
    }
    node.inline_image_source()
        .map(|_| Key::Node(node.id().to_owned()))
}
impl Inline {
    pub fn from_mount(current: &MountedSurface, id: &str) -> Result<Self, ResourceError> {
        if current.snapshot.get().len() > 1024 * 1024 {
            return Err(ResourceError::Limit);
        }
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Snapshot {
            protocol_version: String,
            instance_id: String,
            attachment_id: String,
            revision: u64,
            scene: Box<serde_json::value::RawValue>,
        }
        let snapshot: Snapshot =
            serde_json::from_str(current.snapshot.get()).map_err(|_| ResourceError::Input)?;
        if snapshot.protocol_version != "loom.surface.v1"
            || snapshot.instance_id != current.binding.instance()
            || snapshot.attachment_id != current.binding.attachment()
            || snapshot.revision == 0
        {
            return Err(ResourceError::Identity);
        }
        let scene =
            Scene::parse(snapshot.scene.get().as_bytes()).map_err(|_| ResourceError::Input)?;
        fn find<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
            if !node.visible() {
                return None;
            }
            if node.id() == id {
                return Some(node);
            }
            node.visible_children().find_map(|child| find(child, id))
        }
        let node = find(scene.root(), id).ok_or(ResourceError::Missing)?;
        if key(node) != Some(Key::Node(id.to_owned())) {
            return Err(ResourceError::Identity);
        }
        let source = node
            .props()
            .get("src")
            .and_then(|v| v.as_str())
            .ok_or(ResourceError::Input)?;
        let mime = super::data::mime(source)?;
        Ok(Self {
            binding: current.binding.clone(),
            revision: snapshot.revision,
            node: id.to_owned(),
            source: Arc::from(source),
            mime,
            snapshot_digest: digest(current),
        })
    }
    pub(super) fn matches_digest(
        &self,
        current: &MountedSurface,
        digest: &[u8; 32],
    ) -> Result<(), ResourceError> {
        if self.binding != current.binding || &self.snapshot_digest != digest {
            return Err(ResourceError::Identity);
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn matches(&self, current: &MountedSurface) -> Result<(), ResourceError> {
        self.matches_digest(current, &digest(current))
    }
    pub(super) fn binding(&self) -> &Binding {
        &self.binding
    }
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
    pub(super) fn key(&self) -> Key {
        Key::Node(self.node.clone())
    }
    pub(super) fn mime(&self) -> &str {
        &self.mime
    }
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
}
/// 原始快照不可变身份；同 revision 偷换来源、可见性或节点也会失效。
pub(crate) fn digest(current: &MountedSurface) -> [u8; 32] {
    Sha256::digest(current.snapshot.get().as_bytes()).into()
}
