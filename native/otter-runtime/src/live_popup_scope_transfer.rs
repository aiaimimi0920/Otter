//! Original-source snapshot from UI to input worker; transfer only kept-alive Targets/values, without rebinding current displays or unsafe Send HWNDs.
use super::*;

pub enum Transfer {
    Owned(OwnedPopup),
    Chained {
        parent: Box<Transfer>,
        child: Box<Transfer>,
        owner: crate::live_hover_owner::Identity,
        epoch: usize,
    },
    Leased {
        root: Target,
        target: Target,
        anchor: Target,
        owner: crate::live_hover_owner::Identity,
        epoch: usize,
        request: usize,
        instance: usize,
        descendant: Option<OwnedPopup>,
    },
}
impl Scope {
    pub fn transfer(&self) -> Transfer {
        match self {
            Self::Owned(scope) => Transfer::Owned(scope.clone()),
            Self::Chained {
                parent,
                child,
                owner,
                epoch,
            } => Transfer::Chained {
                parent: Box::new(parent.transfer()),
                child: Box::new(child.transfer()),
                owner: *owner,
                epoch: *epoch,
            },
            Self::Leased {
                root,
                target,
                anchor,
                owner,
                lease,
                descendant,
            } => Transfer::Leased {
                root: root.clone(),
                target: target.clone(),
                anchor: anchor.clone(),
                owner: *owner,
                epoch: lease.epoch,
                request: lease.request,
                instance: lease.instance,
                descendant: descendant.clone(),
            },
        }
    }
    /// Only for original held/hover/modal cleanup; B/control-epoch revocation does not erase responsibility, and new display instances must not receive old cleanup.
    pub fn cleanup_target(&self) -> Result<Target> {
        match self {
            Self::Owned(scope) => scope.target(),
            Self::Chained { parent, child, .. } => {
                parent.cleanup_target()?;
                child.cleanup_target()
            }
            Self::Leased {
                root,
                target,
                anchor,
                lease,
                descendant,
                ..
            } => {
                root.validate()?;
                target.validate()?;
                anchor.validate()?;
                if !lease.same_instance() {
                    return Err(denied());
                }
                if let Some(descendant) = descendant {
                    descendant.validate()?;
                }
                Ok(target.clone())
            }
        }
    }
}
impl Transfer {
    pub fn into_scope(self) -> Scope {
        match self {
            Self::Owned(scope) => Scope::Owned(scope),
            Self::Chained {
                parent,
                child,
                owner,
                epoch,
            } => Scope::Chained {
                parent: Box::new(parent.into_scope()),
                child: Box::new(child.into_scope()),
                owner,
                epoch,
            },
            Self::Leased {
                root,
                target,
                anchor,
                owner,
                epoch,
                request,
                instance,
                descendant,
            } => {
                // Scope::bind already guarantees these four lease fields match the original Target/Identity.
                // Restore the old snapshot here without reading the current Lease, much less granting it anew to a new display instance.
                let lease = otter_live_hover_hook::native_popup::Lease {
                    popup: anchor.hwnd(),
                    root: root.hwnd(),
                    owner: owner.window(),
                    cookie: owner.cookie,
                    epoch,
                    request,
                    instance,
                };
                Scope::Leased {
                    root,
                    target,
                    anchor,
                    owner,
                    lease,
                    descendant,
                }
            }
        }
    }
}
#[cfg(test)]
#[path = "live_popup_scope_transfer_tests.rs"]
mod tests;
