//! Restore existing Surface attachments required by Live; restore server state only, without claiming completed native-renderer mounting.
use super::*;
use crate::surface_live_binding::{Binding, Error as BindingError};
use serde::Deserialize;
use std::collections::BTreeMap;
pub struct MountedSurface {
    pub binding: Binding,
    /// Raw scene snapshots with verified identity only; the renderer must still validate scene/resources and other business contents.
    pub snapshot: Box<serde_json::value::RawValue>,
}
impl MountedSurface {
    /// Obtain the pre-render scene model without executing it; JS/shader/remote must use their respective runtimes.
    pub fn declarative_scene(
        &self,
    ) -> Result<crate::surface_scene::Scene, crate::surface_scene::Error> {
        scene_for_runtime(
            self.snapshot.get(),
            crate::surface_host_capabilities::Runtime::Declarative,
        )
    }
}
/// The expected runtime must come from the host's original admission, not execution permission inferred from the response awaiting convergence.
pub(crate) fn scene_for_runtime(
    raw: &str,
    expected: crate::surface_host_capabilities::Runtime,
) -> Result<crate::surface_scene::Scene, crate::surface_scene::Error> {
    #[derive(Deserialize)]
    struct SceneSnapshot<'a> {
        #[serde(default)]
        runtime: crate::surface_host_capabilities::Runtime,
        #[serde(borrow)]
        scene: &'a serde_json::value::RawValue,
    }
    let snapshot: SceneSnapshot<'_> =
        serde_json::from_str(raw).map_err(|_| crate::surface_scene::Error::Json)?;
    if snapshot.runtime != expected {
        return Err(crate::surface_scene::Error::Runtime);
    }
    crate::surface_scene::Scene::parse(snapshot.scene.get().as_bytes())
}
#[derive(Deserialize)]
struct Response {
    instance: Instance,
}
#[derive(Deserialize)]
struct Instance {
    attachments: BTreeMap<String, Attachment>,
}
#[derive(Deserialize)]
struct Attachment {
    snapshot: Option<Box<serde_json::value::RawValue>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    protocol_version: String,
    instance_id: String,
    attachment_id: String,
    revision: u64,
}
impl Control<'_> {
    /// Synchronous network method for background owners only; success does not mean UI mounting and does not automatically send active.
    pub fn remount_surface(&self, expected: &Binding) -> Result<MountedSurface, Error> {
        if expected.origin() != self.client.origin()
            || expected.device() != self.session.device_id()
        {
            return Err(Error::SurfaceBinding(BindingError::Identity));
        }
        // IDs come from the binding contract, but still check dot segments to prevent URL normalization from rewriting instance paths.
        if matches!(expected.instance(), "." | "..") {
            return Err(Error::Input);
        }
        let mut url = url::Url::parse("https://path.invalid/").map_err(|_| Error::Input)?;
        url.path_segments_mut()
            .map_err(|_| Error::Input)?
            .clear()
            .extend(["v1", "surfaces", "instances", expected.instance(), "mount"]);
        let body = serde_json::to_vec(&serde_json::json!({"attachmentId":expected.attachment()}))
            .map_err(|_| Error::Input)?;
        let bytes = self.send("POST", url.path(), &body, Duration::from_secs(10))?;
        mounted(&bytes, expected)
    }
}
fn mounted(bytes: &[u8], expected: &Binding) -> Result<MountedSurface, Error> {
    let binding = Binding::from_mount(bytes, expected.origin(), expected.device(), expected.node())
        .map_err(Error::SurfaceBinding)?;
    if binding.instance() != expected.instance()
        || binding.attachment() != expected.attachment()
        || binding.generation() < expected.generation()
    {
        return Err(Error::SurfaceBinding(BindingError::Identity));
    }
    snapshot(bytes, binding)
}
pub(super) fn snapshot(bytes: &[u8], binding: Binding) -> Result<MountedSurface, Error> {
    let mut response: Response = protocol::parse(bytes).map_err(Error::Protocol)?;
    let snapshot = response
        .instance
        .attachments
        .remove(binding.attachment())
        .and_then(|attachment| attachment.snapshot)
        .ok_or(Error::SurfaceBinding(BindingError::Missing))?;
    let header: Header = serde_json::from_str(snapshot.get())
        .map_err(|_| Error::SurfaceBinding(BindingError::Json))?;
    if header.protocol_version != "loom.surface.v1"
        || header.instance_id != binding.instance()
        || header.attachment_id != binding.attachment()
        || header.revision == 0
    {
        return Err(Error::SurfaceBinding(BindingError::Identity));
    }
    Ok(MountedSurface { binding, snapshot })
}
