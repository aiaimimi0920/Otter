//! Serial configure/claim/ACK inside the original Live control worker; share the keyboard/mouse actor cursor without extra network threads.
use super::*;
use crate::art_live_trigger::{
    queue::{Command, Registration},
    Status,
};
use serde_json::json;
impl Sender {
    pub(super) fn drive_local_trigger(
        &mut self,
        control: &Control<'_>,
        target: &LocalArtTarget,
        owner: &mut Owner,
    ) -> Result<(), HttpError> {
        let mut stop_attempted = false;
        let result = self.local_trigger_step(control, target, owner, &mut stop_attempted);
        if result.is_err() {
            let queue = &self.queue.local_triggers;
            if let Some(stop) = queue.reject_terminal(!stop_attempted) {
                // This is the separately requested reduction, not a retry of the failed POST.
                // Preserve the original failure and do not claim the remote stop was confirmed.
                let _ = Self::send_local_stop(control, target, owner, &stop);
            }
        }
        result
    }
    fn send_local_stop(
        control: &Control<'_>,
        target: &LocalArtTarget,
        owner: &Owner,
        r: &Registration,
    ) -> Result<(), HttpError> {
        match control.local_art_trigger(
            owner.identity(),
            &target.proof,
            &r.scope,
            &r.condition,
            json!({"kind":"stop","scope":r.scope}),
            true,
        ) {
            Ok(reply) if reply.status == Status::Stopped && !reply.enabled => Ok(()),
            Err(HttpError::Status(404)) => Ok(()),
            Err(error) => Err(error),
            _ => Err(HttpError::Input),
        }
    }
    fn local_trigger_step(
        &mut self,
        control: &Control<'_>,
        target: &LocalArtTarget,
        owner: &mut Owner,
        stop_attempted: &mut bool,
    ) -> Result<(), HttpError> {
        let queue = &self.queue.local_triggers;
        queue.expire();
        if target.proof.validate().is_err() {
            queue.close();
            return Ok(());
        }
        if let Some(command) = queue.take_command() {
            match command {
                Command::Stop(r) => {
                    *stop_attempted = true;
                    Self::send_local_stop(control, target, owner, &r)?;
                    queue.confirm(&r, false);
                }
                Command::Configure(r) => {
                    let next = self.next()?;
                    let (before, _) = control.trigger_snapshot(owner.identity())?;
                    if before != self.cursor {
                        return Err(HttpError::Cursor);
                    }
                    let sent=control.local_art_trigger(owner.identity(),&target.proof,&r.scope,&r.condition,
                        json!({"kind":"configure","configuration":{"scope":r.scope,"consent":"automatic_local_callback",
                            "condition":r.condition,"sequence":next}}),false);
                    let (after, _) = control.trigger_snapshot(owner.identity())?;
                    let reply = match sent {
                        Ok(reply) => Some(reply),
                        Err(HttpError::Status(400..=499)) if after == before => {
                            queue.reject_configure(&r);
                            self.queue.notify_trigger();
                            return Ok(());
                        }
                        Err(_) => control
                            .local_art_trigger(
                                owner.identity(),
                                &target.proof,
                                &r.scope,
                                &r.condition,
                                json!({"kind":"inspect","scope":r.scope}),
                                true,
                            )
                            .ok(),
                    };
                    if after
                        != (RequesterCursor {
                            control: next,
                            input: before.input,
                        })
                        || !reply.is_some_and(|r| {
                            r.enabled && r.status == Status::Idle && r.delivery.is_none()
                        })
                    {
                        return Err(HttpError::Cursor);
                    }
                    self.cursor.control = next;
                    queue.confirm(&r, true);
                }
            }
            self.queue.notify_trigger();
        }
        if let Some((r, ticket, receipt)) = queue.take_receipt() {
            let delivery_id = &ticket.event().delivery.delivery_id;
            let reply = control.local_art_trigger(
                owner.identity(),
                &target.proof,
                &r.scope,
                &r.condition,
                json!({"kind":"ack","scope":r.scope,"deliveryId":delivery_id,"receipt":receipt}),
                false,
            )?;
            if !reply.confirms_ack(delivery_id, &receipt) {
                return Err(HttpError::Input);
            }
            queue.acknowledged();
            self.queue.notify_trigger();
        }
        if Instant::now() < self.local_poll {
            return Ok(());
        }
        self.local_poll = Instant::now() + Duration::from_millis(750);
        let Some(r) = queue.poll_registration() else {
            return Ok(());
        };
        let reply = control.local_art_trigger(
            owner.identity(),
            &target.proof,
            &r.scope,
            &r.condition,
            json!({"kind":"claim","scope":r.scope}),
            false,
        )?;
        match (reply.status, reply.delivery) {
            (Status::Idle | Status::Acknowledged, None) => {}
            (Status::Stopped | Status::Expired, None) => queue.retire(),
            (Status::Claimed, Some(delivery)) => {
                queue.publish(&r, delivery);
            }
            // Claimed without payload means an unknown claim; never reconstruct execution from Inspect or old observations.
            _ => return Err(HttpError::Cursor),
        }
        self.queue.notify_trigger();
        Ok(())
    }
}
