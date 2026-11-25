//! Check all write-ahead records on each workspace/source check; preserve failed intent and retry unknown acceptance only through explicit import.
use super::*;
impl Application {
    pub(super) unsafe fn start_account_recovery(&mut self) -> Result<bool> {
        let Some(scope) = self
            .restored_session
            .as_ref()
            .and_then(|r| r.projection_scope)
        else {
            return Ok(false);
        };
        let Some(origin) = self.authorization.borrow().origin.clone() else {
            return Ok(false);
        };
        let key = (scope, origin.clone());
        if self.account_projection.recovery.as_ref() == Some(&key) {
            if self.account_projection.recovery_retry_at.is_some()
                || self
                    .authorization
                    .borrow()
                    .consent
                    .participation()
                    .is_none()
            {
                return Ok(false);
            }
            if let Some(intent) = self.account_projection.recovery_queue.front().cloned() {
                let started = self.start_account_request(
                    crate::desktop::device_authorization_ui::account::Request::Recover(intent),
                );
                return match settle_local_start(
                    &mut self.account_projection.recovery_queue,
                    started,
                ) {
                    Ok(started) => Ok(started),
                    Err(error) => {
                        // A locally failing intent must not starve sync of existing bindings.
                        self.account_projection.recovery_retry_at =
                            Some(Instant::now() + Duration::from_secs(5));
                        self.account_status(&crate::ui_text::format(
                            "app-account-recovery-check-failed",
                            &[("error", error.to_string().into())],
                        ));
                        Ok(false)
                    }
                };
            }
            return Ok(false);
        }
        let participation = self
            .authorization
            .borrow()
            .consent
            .participation()
            .ok_or_else(stale)?;
        let Some(owner) = self.workspace_checkpoint.as_ref() else {
            return Ok(false);
        };
        let root = owner.profile_root().to_owned();
        let bindings = owner.account_bindings(&self.workspace);
        let destination = account_projection::Destination::capture_existing(self)?;
        let source = destination.source().clone();
        let units = self.workspace.units().to_vec();
        let lease = self
            .authorization_owner
            .lease(&origin, crate::desktop::surface_windows::now_ms()?)
            .map_err(|_| stale())?;
        let selected = origin.clone();
        let job = job::Job::start(lease, self.execution_wake.callback(), move |lease| {
            let cancel = lease.preparation_cancel();
            let mut store = account::journal::Store::open(&root, cancel).map_err(job::failure)?;
            let entries = store.entries().to_vec();
            let mut recover = Vec::new();
            for intent in entries {
                if intent.scope != scope || intent.loom_origin != selected {
                    continue;
                }
                if bindings
                    .iter()
                    .any(|binding| binding.stopped() && intent.matches(binding))
                {
                    // Only committed stop records may retire; network failures and cancellation never reach this path.
                    store.retire(&intent, cancel).map_err(job::failure)?;
                } else if !units.iter().any(|unit| unit.as_str() == intent.unit) {
                    recover.push(intent);
                }
            }
            Ok(Output::Recovery(recover))
        })?;
        self.account_projection.pending = Some(Pending {
            job,
            origin,
            source,
            workspace: self.workspace.identity_stamp(),
            guard: Guard::Receive(Box::new(destination)),
            cancel: None,
            participation: Some(participation),
        });
        self.account_status(crate::ui_text::text("app-account-checking"));
        Ok(true)
    }
}

// Err from start_account_request means no worker was spawned. Keep the item,
// but rotate it so one locally failing intent does not block later intents.
// Once a worker is admitted, poll owns completion; do not requeue its errors.
fn settle_local_start<T, E>(
    queue: &mut std::collections::VecDeque<T>,
    started: std::result::Result<(), E>,
) -> std::result::Result<bool, E> {
    match started {
        Ok(()) => {
            queue.pop_front();
            Ok(true)
        }
        Err(error) => {
            queue.rotate_left(1);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{settle_local_start, Owner};
    use std::{
        collections::VecDeque,
        time::{Duration, Instant},
    };

    #[test]
    fn account_recovery_deadline_does_not_block_existing_sync() {
        let now = Instant::now();
        let deadline = now + Duration::from_secs(5);
        let mut owner = Owner {
            due: Some(now + Duration::from_secs(1)),
            recovery_retry_at: Some(deadline),
            ..Owner::default()
        };
        assert_eq!(owner.deadline(), owner.due);
        owner.expire_deadlines(now + Duration::from_secs(1));
        assert_eq!(owner.due, None);
        assert_eq!(owner.deadline(), Some(deadline));
        owner.expire_deadlines(deadline);
        assert_eq!(owner.deadline(), None);
    }

    #[test]
    fn account_recovery_queue_preserves_failed_local_start_and_advances_other_items() {
        let mut queue = VecDeque::from(["first", "second"]);
        assert_eq!(
            settle_local_start(&mut queue, Err("not started")),
            Err("not started")
        );
        assert_eq!(queue, ["second", "first"]);
        assert_eq!(settle_local_start::<_, &str>(&mut queue, Ok(())), Ok(true));
        assert_eq!(queue, ["first"]);
        assert_eq!(
            settle_local_start(&mut queue, Err("still deferred")),
            Err("still deferred")
        );
        assert_eq!(queue, ["first"]);
        assert_eq!(settle_local_start::<_, &str>(&mut queue, Ok(())), Ok(true));
        assert!(queue.is_empty());
    }
}
