//! 菜单中的索引必须绑定原目录/收件箱/工作区；同名条目不能替换已打开弹层的命令目标。
use super::*;
use std::sync::Arc;
pub(in crate::desktop) struct Snapshot {
    document: crate::desktop::popup_menu::target::MenuTarget,
    origin: Option<String>,
    directory: Option<Arc<crate::projection_delivery::targets::Directory>>,
    invitations: Vec<Arc<crate::projection_delivery::Invitation>>,
    recovery: Vec<Arc<crate::projection_delivery::journal::Ticket>>,
    saved: Vec<Arc<crate::projection_delivery::journal::Ticket>>,
    source: Option<crate::session_source::Source>,
    bindings: Vec<crate::projection_delivery::sender::binding::Binding>,
    account_bindings: Vec<crate::projection_account::scene::Binding>,
    live: Vec<std::sync::Arc<crate::desktop::composition::relay::sessions::ViewerKey>>,
    editing: Option<crate::desktop::projection_edit_workspace::Stamp>,
}
fn same<T>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}
fn same_list<T>(a: &[Arc<T>], b: &[Arc<T>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Arc::ptr_eq(a, b))
}
impl Snapshot {
    pub(in crate::desktop) fn capture(state: &State) -> Self {
        let view = state.authorization.borrow();
        Self {
            document: crate::desktop::popup_menu::target::MenuTarget::capture(
                &state.canvas.borrow(),
            ),
            origin: view.origin.clone(),
            directory: view.targets.directory.clone(),
            invitations: view.projection.entries.clone(),
            recovery: view.projection.recovery.entries.clone(),
            saved: view.projection.recovery.saved.clone(),
            source: view.sender.stop_source.clone(),
            bindings: sender::stops(&view, &state.unit_id).cloned().collect(),
            account_bindings: view
                .account
                .bindings
                .iter()
                .filter(|binding| binding.unit() == state.unit_id.as_str())
                .cloned()
                .collect(),
            live: view.live.watching.iter().map(|v| v.key.clone()).collect(),
            editing: state.projection_edit.borrow().as_ref().map(|o| o.stamp()),
        }
    }
    pub(in crate::desktop) fn matches(&self, state: &State) -> bool {
        !state.authorization.borrow().closed
            && !state.modals.blocked()
            && !state.exit_requested.get()
            && !state.group_locked.get()
            && !state.group_hidden.get()
            && crate::desktop::pin_settings::is_open(state)
            && self.same_inputs(state)
    }
    // 刷新持有子栏可变借用；只比较业务输入，不递归借用子栏的 open 状态。
    pub(in crate::desktop) fn same_inputs(&self, state: &State) -> bool {
        let view = state.authorization.borrow();
        !view.closed
            && self.document.matches(&state.canvas.borrow())
            && self.live.len() == view.live.watching.len()
            && self
                .live
                .iter()
                .zip(&view.live.watching)
                .all(|(key, viewer)| Arc::ptr_eq(key, &viewer.key))
            && self.origin == view.origin
            && same(&self.directory, &view.targets.directory)
            && same_list(&self.invitations, &view.projection.entries)
            && same_list(&self.recovery, &view.projection.recovery.entries)
            && same_list(&self.saved, &view.projection.recovery.saved)
            && self.account_bindings.iter().eq(view
                .account
                .bindings
                .iter()
                .filter(|binding| binding.unit() == state.unit_id.as_str()))
            && self
                .bindings
                .iter()
                .eq(sender::stops(&view, &state.unit_id))
            && match (&self.source, &view.sender.stop_source) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_storage(b),
                _ => false,
            }
            && self.editing == state.projection_edit.borrow().as_ref().map(|o| o.stamp())
    }
}
