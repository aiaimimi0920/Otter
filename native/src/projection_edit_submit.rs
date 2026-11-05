//! 已有协作会话的单批Apply；复用原Job/Slot/Lease，网络前持久化，HTTP后仍保留pending。
use super::*;
use crate::projection_edit::{
    self as protocol,
    journal::{Store, Ticket},
    view::View,
    workspace::{Role, Snapshot},
    Document, Failure, Mode, Objects, Request,
};
use std::path::PathBuf;
#[path = "projection_edit_resolution.rs"]
mod resolution;
#[path = "projection_edit_source_flow.rs"]
pub(crate) mod source;
pub(crate) use resolution::{Action, Resolution};
#[path = "projection_edit_prepare.rs"]
mod preparation;
pub(crate) use preparation::automatic;

pub(crate) struct Input {
    pub root: PathBuf,
    pub snapshot: Snapshot,
    pub op_id: String,
    pub action: Action,
}
pub struct Ready {
    pub(crate) ticket: Option<Ticket>,
    pub(crate) document: Document,
    pub(crate) resolution: Option<Resolution>,
    pub(crate) rejection: Option<protocol::Code>,
    pub(crate) source: Option<source::Ready>,
    _store: Store,
}
impl std::fmt::Debug for Ready {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionEditReady")
            .field("rejection", &self.rejection)
            .field("resolution", &self.resolution.is_some())
            .finish_non_exhaustive()
    }
}
impl Job {
    pub(crate) fn start_edit(config: Config, lease: Lease, input: Input) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::Edit(Box::new(input)),
            Some(Slot::acquire()?),
            None,
        )
    }
    pub(crate) fn start_edit_ack(
        config: Config,
        lease: Lease,
        ready: Box<Ready>,
    ) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::EditAck(ready),
            Some(Slot::acquire()?),
            None,
        )
    }
}
impl Input {
    pub(super) fn run(
        self,
        control: &crate::live_relay_http::Control<'_>,
        lease: &Lease,
    ) -> Result<Completed, Error> {
        let result = (|| {
            let cancel = lease.preparation_cancel();
            if self.snapshot.context().device() != lease.device_id()
                || self.snapshot.suspended().is_some()
            {
                return Err(Failure::Protocol(protocol::Error::Identity));
            }
            let store = Store::open(&self.root, cancel).map_err(Failure::Journal)?;
            if let Action::Resolve { keep_local } = self.action {
                return resolution::read(self.snapshot, keep_local, store, control, cancel);
            }
            let pending = store.pending(cancel).map_err(Failure::Journal)?;
            let ticket = if let Some(ticket) = pending
                .into_iter()
                .find(|ticket| ticket.context().same_key(self.snapshot.context()))
            {
                if matches!(self.action, Action::Automatic | Action::Mode(_)) {
                    return Err(Failure::Journal(protocol::journal::Error::Busy));
                }
                if ticket.context() != self.snapshot.context()
                    || ticket.route() != self.snapshot.route()
                    || ticket.request().projection_id() != self.snapshot.projection_id()
                    || !matches!(
                        ticket.request().operation(),
                        protocol::Operation::Apply { .. }
                            | protocol::Operation::Mode { .. }
                            | protocol::Operation::Checkpoint { .. }
                    )
                    || ticket
                        .previous()
                        .is_none_or(|previous| self.snapshot.document().follows(previous).is_err())
                {
                    return Err(Failure::Protocol(protocol::Error::Identity));
                }
                ticket // 显式重试只恢复原opId/基线/32项；不能用当前Editor重造未知副作用。
            } else {
                let (request, acknowledged) =
                    preparation::request(&self.snapshot, self.op_id, self.action)?;
                store
                    .begin_observed(
                        self.snapshot.context().clone(),
                        self.snapshot.route(),
                        request,
                        self.snapshot.document().clone(),
                        acknowledged,
                        cancel,
                    )
                    .map_err(Failure::Journal)?
            };
            let ticket = if ticket.rejection().is_some() {
                ticket // 明确拒绝已落盘，重开/显式重试只read，不再发旧Apply。
            } else {
                match control.projection_edit(&ticket) {
                    Ok(document) => {
                        return Ok(Ready {
                            ticket: Some(ticket),
                            document,
                            resolution: None,
                            rejection: None,
                            source: None,
                            _store: store,
                        })
                    }
                    Err(Failure::Rejected { code, .. }) if code.recoverable() => {
                        ticket.reject(code, cancel).map_err(Failure::Journal)?
                    }
                    Err(error) => return Err(error),
                }
            };
            let document = control.read_projection_edit(
                self.snapshot.context(),
                self.snapshot.projection_id(),
                self.snapshot.route(),
                Some(self.snapshot.document()),
            )?;
            let rejection = ticket.rejection();
            Ok(Ready {
                ticket: Some(ticket),
                document,
                resolution: None,
                rejection,
                source: None,
                _store: store,
            })
        })();
        result
            .map(|ready| Completed::Metadata(Output::Edited(Box::new(ready))))
            .map_err(Error::Edit)
    }
}
fn prepare(snapshot: &Snapshot, op_id: String) -> Result<(Request, Objects), Failure> {
    let fail = || Failure::Protocol(protocol::Error::Response);
    if snapshot.conflict().is_some()
        || snapshot.suspended().is_some()
        || (snapshot.role() == Role::Receiver && snapshot.document().mode() != Mode::TwoWay)
    {
        return Err(fail());
    }
    let current = snapshot.editor_view().map_err(|_| fail())?;
    let observed = View::from_document(snapshot.document(), current.size()).map_err(|_| fail())?;
    let mut changes = observed.changes_to(&current).map_err(|_| fail())?;
    changes.truncate(32);
    let acknowledged = observed.overlay(&changes).map_err(|_| fail())?;
    let request = Request::prepare(protocol::Operation::Apply {
        projection_id: snapshot.projection_id().into(),
        session_id: snapshot.document().session_id().into(),
        op_id,
        base_revision: snapshot.document().revision(),
        mode_revision: snapshot.document().mode_revision(),
        changes,
    })
    .map_err(Failure::Protocol)?;
    Ok((request, acknowledged.basis_objects().clone()))
}
#[cfg(test)]
#[path = "projection_edit_submit_tests.rs"]
mod tests;
