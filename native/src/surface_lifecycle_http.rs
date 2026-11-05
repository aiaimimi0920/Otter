//! 生命周期 HTTP 仅在后台调用；active 必须由实际 UI 挂载确认触发，不由 remount 自动上报。
use super::*;
use crate::{surface_lifecycle_owner::Owner, surface_live_binding::Lifecycle};
impl Control<'_> {
    pub fn transition_surface(&self, owner: &mut Owner, state: Lifecycle) -> Result<(), Error> {
        let binding = owner.binding();
        if binding.origin() != self.client.origin() || binding.device() != self.session.device_id()
        {
            return Err(Error::SurfaceBinding(
                crate::surface_live_binding::Error::Identity,
            ));
        }
        if matches!(binding.instance(), "." | "..") {
            return Err(Error::Input);
        }
        let mut url = url::Url::parse("https://path.invalid/").map_err(|_| Error::Input)?;
        url.path_segments_mut()
            .map_err(|_| Error::Input)?
            .clear()
            .extend([
                "v1",
                "surfaces",
                "instances",
                binding.instance(),
                "lifecycle",
            ]);
        let instance = binding.instance().to_owned();
        let attachment = binding.attachment().to_owned();
        let revision = owner.begin(state).map_err(Error::SurfaceLifecycle)?;
        let body = serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface.v1",
            "instanceId":instance,"attachmentId":attachment,"state":state,"revision":revision}))
        .map_err(|_| Error::Input)?;
        // 这里开始任一错误均保留 unknown；不根据 HTTP 错误猜测远端是否已经提交。
        let bytes = self.send("POST", url.path(), &body, Duration::from_secs(10))?;
        owner
            .acknowledge(&bytes, state, revision)
            .map_err(Error::SurfaceLifecycle)
    }
    pub fn recover_surface(&self, owner: &mut Owner) -> Result<MountedSurface, Error> {
        if owner.disposed() {
            return Err(Error::SurfaceLifecycle(
                crate::surface_lifecycle_owner::Error::Disposed,
            ));
        }
        if owner.binding().origin() != self.client.origin()
            || owner.binding().device() != self.session.device_id()
        {
            return Err(Error::SurfaceBinding(
                crate::surface_live_binding::Error::Identity,
            ));
        }
        owner.begin_recovery().map_err(Error::SurfaceLifecycle)?;
        let mounted = self.remount_surface(owner.binding())?;
        owner
            .recover(mounted.binding.clone())
            .map_err(Error::SurfaceLifecycle)?;
        Ok(mounted)
    }
}
