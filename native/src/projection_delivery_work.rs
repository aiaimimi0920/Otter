//! Protocol dispatch for the original serial network Job; displayed is constructed only by continuation retaining the original Hold.
use super::*;
pub(super) enum Work {
    Request(Operation),
    Displayed(Box<crate::projection_delivery::journal::Ticket>),
    Recover(Box<recovery::Request>),
    Sync(Box<sync::Request>),
    Sender(SenderPreparation),
    SenderUpdate(Box<sender_update::Request>),
    Edit(Box<edit::Input>),
    EditSource(Box<edit::source::Input>),
    EditAck(Box<edit::Ready>),
}
impl Work {
    pub(super) fn run(
        self,
        control: &crate::live_relay_http::Control<'_>,
        client: &Client,
        task: &Arc<Lease>,
        attempt: &attempt::Attempt,
        sender_attempt: &sender_flow::Attempt,
    ) -> Result<Completed, Error> {
        let output = match self {
            Self::Request(Operation::Import { request, journal }) => {
                if !request.current() {
                    return Err(Error::Stopped);
                }
                let invitation = control
                    .inspect_projection(&request, &request.cancel)
                    .map_err(Error::Http)?;
                if !request.current() || task.is_revoked() {
                    return Err(Error::Stopped);
                }
                // Read-only reconstruction of existing remote receiver associations cannot gain fresh-accept automatic-unlink eligibility.
                let intent = if invitation.accepted() {
                    attempt.reattach(&journal, &invitation, task.device_id())?
                } else {
                    attempt.begin(&journal, &invitation, task.device_id())?
                };
                if !request.current() {
                    return Err(Error::Stopped);
                }
                let received = if invitation.accepted() {
                    control.read_projection(&intent)
                } else {
                    control.accept_projection(&intent.invitation())
                }
                .map_err(Error::Http)?;
                return Ok(Completed::Accepted(Box::new(received::decode(
                    received, task, &intent,
                )?)));
            }
            Self::Request(Operation::Send(request)) => {
                return sender_flow::send(&request, control, task)
                    .map(|connected| Completed::Sent(Box::new(connected)));
            }
            Self::Sender(prepare) => {
                return sender_flow::run(prepare, control, task, sender_attempt)
            }
            Self::SenderUpdate(request) => return request.run(control, task),
            Self::Edit(input) => return input.run(control, task),
            Self::EditSource(input) => return input.run(control, task),
            Self::EditAck(_) => unreachable!("local ack handled before preparing client"),
            Self::Request(Operation::CancelSend(request)) => {
                request.cancel().map_err(Error::Sender)?;
                let active = request
                    .activate(task.clone(), true)
                    .map_err(Error::Sender)?;
                control.unlink_sender(&active).map_err(Error::Http)?;
                Output::SenderUnlinked
            }
            Self::Sync(request) => return request.run(control, client, task, attempt),
            Self::Recover(request) => return request.run(control, client, task, attempt),
            Self::Displayed(intent) => {
                control.displayed_projection(&intent).map_err(Error::Http)?;
                // displayed is not unlink; retain journals/saved images for recovery/ongoing association.
                Output::Displayed
            }
            Self::Request(Operation::Targets) => {
                Output::Targets(control.projection_targets().map_err(Error::Http)?)
            }
            Self::Request(Operation::Inbox(policy)) => {
                Output::Inbox(control.projection_inbox(policy).map_err(Error::Http)?)
            }
            Self::Request(Operation::Reject(invitation)) => {
                control
                    .reject_projection(&invitation)
                    .map_err(Error::Http)?;
                Output::Rejected
            }
            Self::Request(Operation::Accept {
                invitation,
                journal,
            }) => {
                // Write-ahead only validates incoming data; request bodies must come from the potentially reused original persistent intent.
                invitation
                    .validate_for(client.origin())
                    .map_err(Error::Decode)?;
                let intent = attempt.begin(&journal, &invitation, task.device_id())?;
                // HTTP accept is permitted only after successful write-ahead intent; failure/revocation cannot erase uncertain side effects.
                let received = match control.accept_projection(&intent.invitation()) {
                    Err(HttpError::ProjectionBusy) => {
                        return Err(if attempt.discard_unapplied()? {
                            Error::AcceptBusyUnapplied
                        } else {
                            Error::Http(HttpError::ProjectionBusy)
                        });
                    }
                    other => other.map_err(Error::Http)?,
                };
                return Ok(Completed::Accepted(Box::new(received::decode(
                    received, task, &intent,
                )?)));
            }
            Self::Request(Operation::Reattach {
                invitation,
                journal,
            }) => {
                let intent = attempt.reattach(&journal, &invitation, task.device_id())?;
                let received = control.read_projection(&intent).map_err(Error::Http)?;
                return Ok(Completed::Accepted(Box::new(received::decode(
                    received, task, &intent,
                )?)));
            }
            Self::Request(Operation::Compensate(mut intent)) => {
                if intent.device() != task.device_id() || intent.origin() != client.origin() {
                    return Err(Error::Stopped);
                }
                intent.activate().map_err(Error::Journal)?;
                control
                    .unlink_received_projection(&intent)
                    .map_err(Error::Http)?;
                (*intent).retire().map_err(Error::Journal)?;
                Output::Compensated
            }
        };
        Ok(Completed::Metadata(output))
    }
}
