//! 显式选择先只读最新权威；未知pending不被read、模式变化或用户选择退休。
use super::*;
use std::sync::atomic::AtomicBool;
#[derive(Clone, Copy)]
pub(crate) enum Action {
    Submit,
    Automatic,
    Mode(Mode),
    Resolve { keep_local: bool },
}
#[derive(Debug)]
pub(crate) struct Resolution {
    pub keep_local: bool,
    pub observed: View,
}
pub(super) fn read(
    snapshot: Snapshot,
    keep_local: bool,
    store: Store,
    control: &crate::live_relay_http::Control<'_>,
    cancel: &AtomicBool,
) -> Result<Ready, Failure> {
    if snapshot.conflict().is_none() || snapshot.suspended().is_some() {
        return Err(Failure::Protocol(protocol::Error::Response));
    }
    if store
        .pending(cancel)
        .map_err(Failure::Journal)?
        .iter()
        .any(|ticket| ticket.context().same_key(snapshot.context()))
    {
        return Err(Failure::Journal(protocol::journal::Error::Busy));
    }
    let document = control.read_projection_edit(
        snapshot.context(),
        snapshot.projection_id(),
        snapshot.route(),
        Some(snapshot.document()),
    )?;
    if keep_local && snapshot.role() == Role::Receiver && document.mode() != Mode::TwoWay {
        return Err(Failure::Rejected {
            status: 403,
            code: protocol::Code::ReadOnly,
        });
    }
    let observed = snapshot
        .editor_view()
        .map_err(|_| Failure::Protocol(protocol::Error::Response))?;
    Ok(Ready {
        ticket: None,
        document,
        resolution: Some(Resolution {
            keep_local,
            observed,
        }),
        rejection: None,
        source: None,
        _store: store,
    })
}
impl Ready {
    pub(crate) fn ack(self, lease: &Lease) -> Result<(), Error> {
        if lease.is_revoked() {
            return Err(Error::Stopped);
        }
        if let Some(source) = &self.source {
            source.check().map_err(Error::Sender)?;
        }
        let Some(ticket) = self.ticket else {
            if let Some(source) = &self.source {
                self._store
                    .observe(&source.context, self.document, lease.preparation_cancel())
                    .map_err(|error| Error::Edit(Failure::Journal(error)))?;
            }
            return Ok(()); // 只读选择没有网络副作用；仍等实际Checkpoint后才释放独占Store。
        };
        let result = if self.rejection.is_some() {
            ticket.commit_rejected_after_checkpoint(self.document, lease.preparation_cancel())
        } else {
            ticket.commit_after_checkpoint(self.document, lease.preparation_cancel())
        };
        result.map_err(|error| Error::Edit(Failure::Journal(error)))
    }
}
