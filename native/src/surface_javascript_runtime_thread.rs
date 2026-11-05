//! 后台独占 Mounted 和 child；空闲等待命令、撤权、child 退出与最早期限，无轮询。
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::WaitForMultipleObjects,
};
pub(super) fn run(
    connection: Connection,
    lease: Arc<Lease>,
    mut current: MountedSurface,
    viewport: Option<crate::surface_viewport::Viewport>,
    shared: &Shared,
) -> Result<(), Error> {
    let mut mounted = prepare(connection, lease, &current, viewport)?;
    shared.set_expires(mounted.expires_at_ms());
    #[cfg(test)]
    {
        *shared.probe.lock().unwrap() = Some(Arc::new(mounted.probe()));
    }
    let frame = mounted.take_initial(&current)?;
    shared
        .publish(Reply {
            sequence: 1,
            binding: current.binding.clone(),
            revision: mounted.entry.revision(),
            frame: Some(frame),
            snapshot: current.snapshot.clone(),
        })
        .map_err(execution)?;
    loop {
        mounted.refresh(&current)?;
        let time = now()?;
        if time >= mounted.expires_at_ms() {
            return Err(Error::Stopped);
        }
        let handles = [
            HANDLE(shared.command_handle().as_raw_handle()),
            HANDLE(mounted.revocation_handle().as_raw_handle()),
            HANDLE(
                mounted
                    .session
                    .as_ref()
                    .and_then(|s| s.child.as_ref())
                    .ok_or(Error::Stopped)?
                    .handle()
                    .as_raw_handle(),
            ),
        ];
        let timeout = (mounted.expires_at_ms() - time).min(u64::from(u32::MAX - 1)) as u32;
        let result = unsafe { WaitForMultipleObjects(&handles, false, timeout) };
        if result == WAIT_TIMEOUT {
            continue;
        }
        if result != WAIT_OBJECT_0 {
            return Err(Error::Stopped);
        }
        let Some((sequence, next, operation)) = shared.take_command().map_err(execution)? else {
            continue;
        };
        let frame = match operation {
            Operation::Rebind => {
                mounted.refresh(&next)?;
                None
            }
            Operation::Click(target, viewport) => {
                mounted
                    .session
                    .as_mut()
                    .ok_or(Error::Stopped)?
                    .set_viewport(viewport)
                    .map_err(execution)?;
                Some(mounted.click(&next, &target)?)
            }
            Operation::LocalInput(input, viewport) => {
                mounted
                    .session
                    .as_mut()
                    .ok_or(Error::Stopped)?
                    .set_viewport(viewport)
                    .map_err(execution)?;
                Some(mounted.local_input(&next, input)?)
            }
            Operation::Update(viewport) => {
                mounted
                    .session
                    .as_mut()
                    .ok_or(Error::Stopped)?
                    .set_viewport(viewport)
                    .map_err(execution)?;
                Some(mounted.update(&next)?)
            }
            Operation::AnimationFrame(viewport) => {
                mounted
                    .session
                    .as_mut()
                    .ok_or(Error::Stopped)?
                    .set_viewport(viewport)
                    .map_err(execution)?;
                Some(mounted.animation_frame(&next)?)
            }
            Operation::TimerTick(viewport) => {
                mounted
                    .session
                    .as_mut()
                    .ok_or(Error::Stopped)?
                    .set_viewport(viewport)
                    .map_err(execution)?;
                Some(mounted.timer_tick(&next)?)
            }
        };
        current = next;
        shared.set_expires(mounted.expires_at_ms());
        shared
            .publish(Reply {
                sequence,
                binding: current.binding.clone(),
                revision: mounted.entry.revision(),
                frame,
                snapshot: current.snapshot.clone(),
            })
            .map_err(execution)?;
    }
}
