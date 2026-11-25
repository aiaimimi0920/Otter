//! Initial Surface mount protocol; do not implicitly enable JavaScript or report active early upon receiving a snapshot.
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
    /// Synchronous networking is background-only. The report comes from the host implementation, not from copying capabilities requested by an Art package.
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
/// Initial mount and candidate recovery use the same Art/runtime contract; do not trust scene contents carried by recovery queries.
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
