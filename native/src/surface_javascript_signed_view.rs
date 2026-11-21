//! 视图更新仍由原 Local::operate 复核许可/撤权；不制造新 revision 或重新挂载实例。
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
