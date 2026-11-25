//! GET recovery in the original serial control thread: same reliable cursor and bounded backoff, without write replay or repeated committed-event consumption.
use super::*;
pub(super) fn poll(
    control: &crate::live_relay_http::Control<'_>,
    lease: &Lease,
    input: &interaction::Queue,
    owner: &mut Owner,
    consume: &mut impl FnMut(&[ControlEvent]) -> Result<(), ()>,
    wait: u16,
) -> Result<crate::live_relay_control_owner::Applied, Exit> {
    let mut attempts = 0;
    let mut applied = None;
    let mut refresh = false;
    loop {
        if lease.is_revoked() || input.intent().1 {
            return Err(Exit::Stopped);
        }
        let result = (|| {
            if applied.is_none() {
                applied = Some(
                    owner
                        .poll_wait(
                            control,
                            |events| {
                                refresh = events.iter().any(|e| e.kind() == "trigger_condition");
                                consume(events)
                            },
                            wait,
                        )
                        .map_err(Exit::Control)?,
                );
            }
            let result = applied.unwrap();
            // After reliable-batch commit, retry only registration GET; never consume the batch again or lose pending registration refresh.
            if !result.closed && refresh {
                let (_, registrations) = control
                    .trigger_snapshot(owner.identity())
                    .map_err(Exit::Http)?;
                owner.registrations(registrations).map_err(Exit::Control)?;
            }
            Ok(result)
        })();
        match result {
            Ok(result) => {
                input.control_recovering(false);
                return Ok(result);
            }
            Err(exit @ (Exit::Control(OwnerError::Http(error)) | Exit::Http(error)))
                if error.retryable_read() =>
            {
                if attempts >= 4 {
                    return Err(exit);
                }
                input.control_recovering(true);
                if input.intent().1
                    || !lease.wait(Duration::from_millis((400u64 << attempts).min(2000)))
                {
                    return Err(Exit::Stopped);
                }
                attempts += 1;
            }
            Err(exit) => return Err(exit),
        }
    }
}
