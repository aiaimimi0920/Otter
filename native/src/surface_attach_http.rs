//! 首次 Surface 挂载协议；不隐式启用 JavaScript，也不在收到快照时提前报告 active。
use super::*;
use crate::{
    surface_host_capabilities::Report,
    surface_live_binding::{Binding, Error as BindingError},
};
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Persistence {
    Temporary,
    Persistent,
}
impl Control<'_> {
    /// 同步网络仅后台调用。report 来自宿主实现，不可从 Art 包的请求能力反向抄写。
    pub fn attach_surface(
        &self,
        art: &str,
        node: &str,
        report: &Report,
        persistence: Persistence,
    ) -> Result<MountedSurface, Error> {
        if !protocol::identifier(art) || !protocol::identifier(node) {
            return Err(Error::Input);
        }
        let body = serde_json::to_vec(&serde_json::json!({"artId":art,"hookNodeId":node,
            "deviceId":self.session.device_id(),"capabilities":report,"persistence":persistence}))
        .map_err(|_| Error::Input)?;
        let bytes = self.send(
            "POST",
            "/v1/surfaces/attach",
            &body,
            Duration::from_secs(10),
        )?;
        let binding =
            Binding::from_mount(&bytes, self.client.origin(), self.session.device_id(), node)
                .map_err(Error::SurfaceBinding)?;
        let mounted = surface::snapshot(&bytes, binding)?;
        validate(&bytes, &mounted, art, report)?;
        Ok(mounted)
    }
}
/// 首次挂载和候选恢复使用相同 Art/runtime 合同，不信任恢复查询携带的场景内容。
pub(super) fn validate(
    bytes: &[u8],
    mounted: &MountedSurface,
    art: &str,
    report: &Report,
) -> Result<(), Error> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Art {
        art_id: String,
    }
    #[derive(serde::Deserialize)]
    struct Response {
        instance: Instance,
    }
    #[derive(serde::Deserialize)]
    struct Instance {
        descriptor: Art,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Scene {
        art_id: String,
        #[serde(default)]
        runtime: crate::surface_host_capabilities::Runtime,
    }
    let outer: Response =
        serde_json::from_slice(bytes).map_err(|_| Error::SurfaceBinding(BindingError::Json))?;
    let returned: Scene = serde_json::from_str(mounted.snapshot.get())
        .map_err(|_| Error::SurfaceBinding(BindingError::Json))?;
    if returned.art_id != art
        || outer.instance.descriptor.art_id != art
        || !report.supports(returned.runtime)
    {
        return Err(Error::SurfaceBinding(BindingError::Identity));
    }
    Ok(())
}
