//! Source控制轮询与媒体独立，使用第二Lease/Cancel；所有输入经单槽桥交回原Session worker。
use super::*;
use crate::{
    device_session_client::Client,
    live_relay_viewer_identity::{ControlCursor, Snapshot},
};
pub(crate) struct Interaction {
    lease: Lease,
    pub bridge: Arc<input::Bridge>,
}
impl Interaction {
    pub fn new(lease: Lease, bridge: Arc<input::Bridge>) -> Arc<Self> {
        Arc::new(Self { lease, bridge })
    }
    pub fn cancel(&self) {
        self.bridge.cancel();
        self.lease.revoke();
    }
    pub(super) fn matches(&self, lease: &Lease) -> bool {
        self.lease.same_grant(lease)
    }
    #[cfg(test)]
    pub(super) fn test_disconnect(&self) -> bool {
        self.lease.test_disconnect_request()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Lease,
    Http(live_relay_http::Error),
    Wire(wire::Error),
    Input(input::Error),
    Panicked,
}
pub(super) fn start(
    client: Client,
    snapshot: Snapshot,
    interaction: Arc<Interaction>,
    signal: Arc<Signal>,
) -> io::Result<JoinHandle<Result<(), Error>>> {
    std::thread::Builder::new()
        .name("otter-live-source-control".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(&client, snapshot, &interaction, &signal)
            }))
            .unwrap_or(Err(Error::Panicked));
            if result.is_err() {
                signal.stop();
            }
            result
        })
}
fn run(
    client: &Client,
    snapshot: Snapshot,
    interaction: &Interaction,
    signal: &Signal,
) -> Result<(), Error> {
    let mut cursor = ControlCursor::new(snapshot.identity().clone(), 0);
    let mut state = wire::State::new(snapshot.revision());
    let mut generation = 0;
    let mut attempts = 0;
    while !signal.stop.load(Ordering::Acquire) && !interaction.bridge.stopping() {
        interaction
            .lease
            .validate(client.origin(), now().map_err(|_| Error::Lease)?)
            .map_err(|_| Error::Lease)?;
        let control = interaction
            .lease
            .control(client)
            .map_err(|_| Error::Lease)?;
        let batch = match control.events(snapshot.identity().session(), &cursor) {
            Ok(batch) => batch,
            Err(_) if signal.stop.load(Ordering::Acquire) => return Ok(()),
            Err(error) if error.retryable_read() && attempts < 4 => {
                if attempts == 0 {
                    interaction
                        .bridge
                        .dispatch(input::Action::Release, || {
                            !signal.stop.load(Ordering::Acquire)
                        })
                        .map_err(Error::Input)?;
                    state.suspend();
                    signal.service_recovery(&signal.control_recovering, true);
                    signal.restart_media();
                }
                let delay = std::time::Duration::from_millis((400u64 << attempts).min(2000));
                attempts += 1;
                if !interaction.lease.wait(delay) {
                    return Ok(());
                }
                continue;
            }
            Err(error) => return Err(Error::Http(error)),
        };
        let _ = release_after_loss(interaction, signal, &mut state, &mut generation)?;
        let (mut next, actions) = state.prepare(batch.events()).map_err(Error::Wire)?;
        let mut blocked = false;
        for action in actions {
            blocked |= release_after_loss(interaction, signal, &mut next, &mut generation)?;
            if (blocked
                || signal.recovering.load(Ordering::Acquire)
                || signal.control_recovering.load(Ordering::Acquire))
                && action != input::Action::Release
            {
                next.suspend();
                continue;
            }
            if let Err(error) = interaction.bridge.dispatch(action, || {
                !signal.stop.load(Ordering::Acquire)
                    && now()
                        .is_ok_and(|now| interaction.lease.validate(client.origin(), now).is_ok())
            }) {
                if signal.stop.load(Ordering::Acquire) {
                    return Ok(());
                }
                return Err(Error::Input(error));
            }
        }
        cursor
            .commit(&batch)
            .map_err(|_| Error::Wire(wire::Error::Protocol))?;
        state = next;
        attempts = 0;
        signal.service_recovery(&signal.control_recovering, false);
        if state.closed() {
            signal.stop();
            return Ok(());
        }
    }
    // UI来源/组状态可先撤销输入桥，即使此后没有新输入，也须结束发布owner。
    signal.stop();
    Ok(())
}
fn release_after_loss(
    interaction: &Interaction,
    signal: &Signal,
    state: &mut wire::State,
    generation: &mut u64,
) -> Result<bool, Error> {
    let current = signal.generation.load(Ordering::Acquire);
    if current != *generation {
        // 只由原串行输入owner释放。旧事件仍验证/提交，但旧controller须经过可靠None屏障。
        interaction
            .bridge
            .dispatch(input::Action::Release, || {
                !signal.stop.load(Ordering::Acquire)
            })
            .map_err(Error::Input)?;
        state.suspend();
        *generation = current;
        return Ok(true);
    }
    Ok(false)
}
#[cfg(test)]
#[path = "live_relay_source_control_tests.rs"]
mod tests;
