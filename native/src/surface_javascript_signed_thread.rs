//! 空闲阻塞等待命令、原 Lease 撤销、child 终态和首次准入期限；不轮询，不在 UI join。
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::WaitForMultipleObjects,
};
fn refresh(
    local: &mut Local,
    current: &Binding,
    online: bool,
    mailbox: &Mailbox,
) -> Result<(), Error> {
    let minimum = mailbox.observed_wall();
    local
        .refresh(current, online, &|| false)
        .map_err(Error::Local)?;
    let observed = local
        .permit
        .lock()
        .map_err(|_| Error::Stopped)?
        .observed_wall();
    if observed < minimum {
        return Err(Error::Stopped);
    }
    mailbox.observe(observed);
    Ok(())
}
pub(super) fn run(
    mount: impl FnOnce() -> Result<(Local, Frame), Error>,
    current: &Binding,
    lease: &Lease,
    deadline: (u64, Instant),
    online: bool,
    mailbox: &Mailbox,
) -> Result<(), Error> {
    let revoked = lease.execution_notification().map_err(io_error)?;
    let (mut local, frame) = mount()?;
    refresh(&mut local, current, online, mailbox)?;
    mailbox
        .publish(Reply { sequence: 1, frame })
        .map_err(io_error)?;
    loop {
        refresh(&mut local, current, online, mailbox)?;
        let time = super::super::now().map_err(Error::Local)?;
        let remaining = deadline.1.saturating_duration_since(Instant::now()).min(
            std::time::Duration::from_millis(deadline.0.saturating_sub(time)),
        );
        if remaining.is_zero() {
            return Err(Error::Stopped);
        }
        let child = local
            .session
            .as_ref()
            .and_then(|session| session.child.as_ref())
            .ok_or(Error::Stopped)?;
        // 撤销/child 终态排在命令前，同步就绪时不再执行排队操作。
        let handles = [
            HANDLE(revoked.handle().as_raw_handle()),
            HANDLE(child.handle().as_raw_handle()),
            HANDLE(mailbox.command_handle().as_raw_handle()),
        ];
        let timeout = remaining
            .as_nanos()
            .div_ceil(1_000_000)
            .min(u128::from(u32::MAX - 1)) as u32;
        let result = unsafe { WaitForMultipleObjects(&handles, false, timeout) };
        if result == WAIT_TIMEOUT {
            continue;
        }
        if result.0 != WAIT_OBJECT_0.0 + 2 {
            return Err(Error::Stopped);
        }
        let Some((sequence, operation)) = mailbox.take_command().map_err(io_error)? else {
            continue;
        };
        refresh(&mut local, current, online, mailbox)?;
        let frame = match operation {
            #[cfg(test)]
            Operation::Click(target) => local.click(current, online, &target, &|| false),
            Operation::Suspend => local.suspend(current, online, &|| false),
            #[cfg(test)]
            Operation::Resume => local.resume(current, online, &|| false),
            Operation::Viewport(viewport) => {
                local.scheduled(current, online, viewport, None, &|| false)
            }
            Operation::Animation(viewport) => {
                local.scheduled(current, online, viewport, Some(true), &|| false)
            }
            Operation::Timer(viewport) => {
                local.scheduled(current, online, viewport, Some(false), &|| false)
            }
            Operation::Input(input, viewport) => local.input(
                current,
                online,
                super::super::Interaction::Input(input),
                viewport,
                &|| false,
            ),
            Operation::ClickAt(target, viewport) => local.input(
                current,
                online,
                super::super::Interaction::Click(target),
                viewport,
                &|| false,
            ),
            Operation::LiveTrigger(ticket, viewport) => {
                local.live_trigger(current, online, ticket, viewport)
            }
        }
        .map_err(Error::Local)?;
        refresh(&mut local, current, online, mailbox)?;
        mailbox
            .publish(Reply { sequence, frame })
            .map_err(io_error)?;
    }
}
