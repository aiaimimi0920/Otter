//! 原串行控制线程的GET恢复：同可靠游标、有限退避，不重放写请求或重复消费已提交事件。
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
            // 可靠批次已提交时只重试注册GET，不能再消费同一批次或遗失待刷新的注册。
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
