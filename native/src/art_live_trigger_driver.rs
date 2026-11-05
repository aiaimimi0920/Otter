//! 原Live控制worker内串行配置/领取/ACK；与键鼠共享actor游标，无额外网络线程。
use super::*;
use crate::art_live_trigger::{queue::Command, Status};
use serde_json::json;
impl Sender {
    pub(super) fn drive_local_trigger(
        &mut self,
        control: &Control<'_>,
        target: &LocalArtTarget,
        owner: &mut Owner,
    ) -> Result<(), HttpError> {
        let result = self.local_trigger_step(control, target, owner);
        if result.is_err() {
            self.queue.local_triggers.reject(true);
        }
        result
    }
    fn local_trigger_step(
        &mut self,
        control: &Control<'_>,
        target: &LocalArtTarget,
        owner: &mut Owner,
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
                    match control.local_art_trigger(
                        owner.identity(),
                        &target.proof,
                        &r.scope,
                        &r.condition,
                        json!({"kind":"stop","scope":r.scope}),
                        true,
                    ) {
                        Ok(reply) if reply.status == Status::Stopped && !reply.enabled => {
                            queue.confirm(&r, false)
                        }
                        Err(HttpError::Status(404)) => queue.confirm(&r, false),
                        Err(error) => return Err(error),
                        _ => return Err(HttpError::Input),
                    }
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
                            queue.reject(false);
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
            let reply=control.local_art_trigger(owner.identity(),&target.proof,&r.scope,&r.condition,
                json!({"kind":"ack","scope":r.scope,"deliveryId":ticket.event().delivery.delivery_id,"receipt":receipt}),false)?;
            if reply.status != Status::Acknowledged
                || reply.receipt.as_ref() != Some(&receipt)
                || reply.delivery.is_some()
            {
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
            // Claimed无payload是未知领取，不允许从Inspect或旧观察重造执行。
            _ => return Err(HttpError::Cursor),
        }
        self.queue.notify_trigger();
        Ok(())
    }
}
