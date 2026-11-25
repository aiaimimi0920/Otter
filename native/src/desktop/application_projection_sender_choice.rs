//! Catalog selection for targeted-device delivery; account sharing uses an independent v2 lifecycle.
use super::*;
use crate::workspace_id::UnitId;
pub(super) enum Choice {
    Device(Selection),
    Invitation(crate::desktop::device_authorization_ui::share::SourceSelection),
}
impl Choice {
    pub(super) fn unit(&self) -> &UnitId {
        match self {
            Self::Device(s) => &s.unit,
            Self::Invitation(s) => &s.unit,
        }
    }
    pub(super) fn origin(&self) -> &str {
        match self {
            Self::Device(s) => s.directory.origin(),
            Self::Invitation(s) => &s.origin,
        }
    }
    pub(super) unsafe fn valid(&self, app: &Application) -> bool {
        match self {
            Self::Invitation(s) => {
                !s.cancel.load(std::sync::atomic::Ordering::Acquire)
                    && app.documents.iter().any(|d| {
                        d.state.unit_id == s.unit
                            && d.is_alive()
                            && s.target.matches(&d.state.canvas.borrow())
                    })
            }
            Self::Device(s) => {
                app.authorization
                    .borrow()
                    .targets
                    .directory
                    .as_ref()
                    .is_some_and(|d| std::sync::Arc::ptr_eq(d, &s.directory))
                    && s.directory
                        .targets()
                        .get(s.index)
                        .is_some_and(|t| t.route().available())
            }
        }
    }
    pub(super) fn name(&self) -> Result<&str> {
        match self {
            Self::Invitation(_) => Ok("局域网邀请"),
            Self::Device(s) => s
                .directory
                .targets()
                .get(s.index)
                .map(|t| t.name())
                .ok_or_else(scope::stale),
        }
    }
    pub(super) fn prepared(
        &self,
        scope_id: &str,
    ) -> Result<crate::projection_delivery::sender::Selection> {
        use crate::projection_delivery::sender::Selection as Prepared;
        match self {
            Self::Invitation(_) => {
                Prepared::invitation(scope_id, self.origin(), self.unit().as_str())
            }
            Self::Device(s) => Prepared::for_target(
                scope_id,
                self.origin(),
                self.unit().as_str(),
                s.directory
                    .targets()
                    .get(s.index)
                    .ok_or_else(scope::stale)?,
            ),
        }
        .map_err(|_| scope::stale())
    }
}
