//! 账号关联的普通产品生命周期；一个后台槽串行网络/候选准备，真实检查点控制显示与后续同步。
use super::*;
use crate::{
    projection_account::{self as account, scene::Binding},
    session_source::Source,
    workspace_id::UnitId,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[path = "application_account_job.rs"]
mod job;
#[path = "application_account_publish.rs"]
mod publish;
#[path = "application_account_recovery.rs"]
mod recovery;
#[path = "application_account_start.rs"]
mod start;
#[path = "application_account_sync.rs"]
mod sync;
#[path = "application_account_work.rs"]
mod work;

#[derive(Default)]
pub(super) struct Owner {
    pending: Option<Pending>,
    saved: Option<Saved>,
    due: Option<Instant>,
    cursor: usize,
    recovery: Option<(crate::session_projection_scope::Scope, String)>,
    recovery_queue: std::collections::VecDeque<account::journal::Intent>,
}
struct Pending {
    job: job::Job,
    origin: String,
    source: Source,
    workspace: crate::workspace::IdentityStamp,
    guard: Guard,
    cancel: Option<Arc<AtomicBool>>,
    participation: Option<crate::desktop::device_authorization_ui::policy::Participation>,
}
enum Guard {
    Source(Box<projection_sender::scope::Scope>),
    Receive(Box<account_projection::Destination>),
    Existing {
        unit: UnitId,
        document: Box<restore_batch::guard::Document>,
    },
}
enum Output {
    Receive(Box<account_projection::Prepared>),
    Patch {
        source: Source,
        binding: Box<Binding>,
        image: Option<crate::projection_delivery_runtime::sync::ImageUpdate>,
    },
    Unchanged,
    Recovery(Vec<account::journal::Intent>),
}
struct Saved {
    binding: Binding,
    workspace: crate::workspace::IdentityStamp,
    source: Source,
    receive: Option<account_projection::Receipt>,
    cancel: Option<Arc<AtomicBool>>,
}
impl Owner {
    pub(super) fn busy(&self) -> bool {
        self.pending.is_some() || self.saved.is_some()
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.due
    }
    pub(super) fn shutdown(&mut self) {
        self.pending.take();
        self.saved.take();
        self.due = None;
        self.recovery_queue.clear();
    }
}
impl Application {
    fn account_idle(&self) -> bool {
        let view = self.authorization.borrow();
        !view.closed
            && !view.busy
            && view.origin.as_deref().is_some_and(|origin| {
                crate::desktop::surface_windows::now_ms()
                    .is_ok_and(|now| self.authorization_owner.token(origin, now).is_some())
            })
            && !matches!(
                view.request,
                Some(
                    crate::desktop::device_authorization_ui::Request::Cancel
                        | crate::desktop::device_authorization_ui::Request::Start(_)
                        | crate::desktop::device_authorization_ui::Request::Select(_)
                        | crate::desktop::device_authorization_ui::Request::ReceivePolicy(_)
                        | crate::desktop::device_authorization_ui::Request::ReceivePause(_)
                )
            )
            && !self.exit_requested.get()
            && !self.modals.blocked()
            && !self.checkpoint_is_exiting()
            && self.restore_job.is_none()
            && self.save_job.is_none()
            && self.projection_checkpoint_available()
            && !self.projection_sender.busy()
            && !self.projection_receive.busy()
            && !self.projection_edit.busy()
            && !self.remote_sessions.projection_pending()
            && restore_batch::guard::is_idle(self)
    }
    fn account_status(&self, text: &str) {
        let mut view = self.authorization.borrow_mut();
        view.account.status = text.into();
        view.account.busy =
            self.account_projection.pending.is_some() || self.account_projection.saved.is_some();
    }
    pub(super) unsafe fn process_account_projection(&mut self) {
        use crate::desktop::device_authorization_ui::Request;
        if self
            .account_projection
            .due
            .is_some_and(|due| due <= Instant::now())
        {
            self.account_projection.due = None;
        }
        self.poll_account_projection();
        let bindings = self
            .workspace_checkpoint
            .as_ref()
            .map(|owner| owner.account_bindings(&self.workspace))
            .unwrap_or_default();
        self.authorization.borrow_mut().account.bindings = bindings.clone();
        let request = {
            let mut view = self.authorization.borrow_mut();
            if matches!(view.request, Some(Request::AccountProjection(_))) {
                view.request.take()
            } else {
                None
            }
        };
        if let Some(Request::AccountProjection(request)) = request {
            if let Err(error) = self.start_account_request(*request) {
                self.account_status(&error.to_string());
            }
            return;
        }
        if self.account_projection.pending.is_some()
            || self.account_projection.saved.is_some()
            || self.account_projection.due.is_some()
            || self.authorization.borrow().request.is_some()
            || !self.account_idle()
        {
            return;
        }
        let origin = self.authorization.borrow().origin.clone();
        match self.start_account_recovery() {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                self.account_status(&format!("账号恢复记录核对失败：{error}"));
                self.account_projection.due = Some(Instant::now() + Duration::from_secs(5));
                return;
            }
        }
        let active: Vec<_> = bindings
            .into_iter()
            .filter(|b| !b.stopped() && Some(b.origin()) == origin.as_deref())
            .collect();
        if active.is_empty() {
            return;
        }
        let next = self.account_projection.cursor % active.len();
        self.account_projection.cursor = next + 1;
        if let Err(error) = self.start_account_sync(active[next].clone(), false) {
            self.account_status(&error.to_string());
            self.account_projection.due = Some(Instant::now() + Duration::from_secs(5));
        }
    }
}
fn stale() -> Error {
    Error::new(
        E_ABORT,
        "Account projection destination changed or unavailable",
    )
}
