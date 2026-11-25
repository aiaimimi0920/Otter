//! 将已持有 Scope 展平为最多四段原始来源；不 bind HWND 或刷新 lease/父控制周期。
use super::*;
use otter_live_hover_hook::native_input_scope::{Link, Snapshot};

impl Scope {
    pub fn native_snapshot(&self) -> Result<Snapshot> {
        self.target()?;
        let mut snapshot = Snapshot::default();
        self.append_native(&mut snapshot, None)?;
        Ok(snapshot)
    }
    fn append_native(
        &self,
        snapshot: &mut Snapshot,
        control: Option<(crate::live_hover_owner::Identity, usize)>,
    ) -> Result<()> {
        let mut link = match self {
            Self::Owned(scope) => Link {
                root: scope.root.identity.window,
                root_thread: scope.root.identity.thread as usize,
                target: scope.popup.identity.window,
                target_thread: scope.popup.identity.thread as usize,
                ..Link::default()
            },
            Self::Chained {
                parent,
                child,
                owner,
                epoch,
            } => {
                // bind_descendant 的 child 只可能是 Owned/Leased；父链按原顺序完整保留。
                if control.is_some() {
                    return Err(denied());
                }
                parent.append_native(snapshot, None)?;
                return child.append_native(snapshot, Some((*owner, *epoch)));
            }
            Self::Leased {
                root,
                target,
                anchor,
                lease,
                ..
            } => Link {
                root: root.identity.window,
                root_thread: root.identity.thread as usize,
                target: target.identity.window,
                target_thread: target.identity.thread as usize,
                owner: lease.owner.0 as usize,
                cookie: lease.cookie,
                epoch: lease.epoch,
                anchor: anchor.identity.window,
                anchor_thread: anchor.identity.thread as usize,
                request: lease.request,
                instance: lease.instance,
            },
        };
        if let Some((owner, epoch)) = control {
            if link.owner != 0
                && (link.owner != owner.hwnd || link.cookie != owner.cookie || link.epoch != epoch)
            {
                return Err(denied());
            }
            link.owner = owner.hwnd;
            link.cookie = owner.cookie;
            link.epoch = epoch;
        }
        if !snapshot.push(link) {
            return Err(denied());
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "live_popup_scope_native_tests.rs"]
mod tests;
