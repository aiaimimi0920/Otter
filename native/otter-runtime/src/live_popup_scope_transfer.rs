//! UI 到输入 worker 的原来源快照；只传保活 Target/数值，不重绑定当前显示或 unsafe Send HWND。
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
    /// 仅供原 held/hover/modal 收尾；B/控制 epoch 撤销不抹去责任，新显示实例不能接收旧清理。
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
                // Scope::bind 已保证这四个 lease 字段与原 Target/Identity 一致。
                // 这里还原旧快照，不读取当前 Lease、更不重新授予新显示实例。
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
