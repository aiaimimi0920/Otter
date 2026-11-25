//! View updates still use original Local::operate permit/revocation checks without fabricating revisions or remounting instances.
use super::*;

impl Local {
    pub(super) fn select_view(
        &mut self,
        current: &wire::Binding,
        online: bool,
        view_id: &crate::art_surface_views::Id,
        viewport: crate::surface_viewport::Viewport,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.set_viewport(viewport)?;
            session.select_view(view_id.as_str(), &stop)
        })
    }
}
