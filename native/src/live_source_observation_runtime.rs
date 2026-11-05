//! Source观察唯一后台owner；单在途child采样/HTTPS，独立同Grant Lease避免与媒体共用取消槽。
use super::*;
use crate::{
    device_session_client::Client, live_source_observation::Tracks,
    live_source_uia::process::Worker, live_window_target::Target,
};
pub(crate) struct Observation {
    lease: Lease,
    target: Target,
}
impl Observation {
    pub(crate) fn new(lease: Lease, target: Target) -> Arc<Self> {
        Arc::new(Self { lease, target })
    }
    pub(super) fn matches(&self, lease: &Lease) -> bool {
        self.lease.same_grant(lease)
    }
    pub(super) fn target_matches(&self, publication: &Publication) -> bool {
        publication.matches_observation(&self.target)
    }
    pub(super) fn cancel(&self) {
        self.lease.revoke();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Lease,
    Source,
    State,
    Http(live_relay_http::Error),
    Spawn,
    Panicked,
}
pub(super) fn start(
    client: Client,
    publication: Arc<Publication>,
    observation: Arc<Observation>,
    signal: Arc<Signal>,
) -> io::Result<JoinHandle<Result<(), Error>>> {
    std::thread::Builder::new()
        .name("otter-live-observation".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(&client, &publication, &observation, &signal)
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
    publication: &Publication,
    observation: &Observation,
    signal: &Signal,
) -> Result<(), Error> {
    let mut worker = None;
    let mut tracks = Tracks::default();
    let stopped = || signal.stop.load(Ordering::Acquire) || observation.lease.is_revoked();
    while !stopped() {
        observation.target.validate().map_err(|_| Error::Source)?;
        observation
            .lease
            .validate(client.origin(), now().map_err(|_| Error::Lease)?)
            .map_err(|_| Error::Lease)?;
        if worker.is_none() {
            worker = Worker::start(observation.target.clone(), &stopped).ok();
        }
        let scan = worker.as_mut().and_then(|w| w.scan(&stopped).ok());
        if stopped() {
            return Ok(());
        }
        let candidates = match scan {
            Some(samples) => tracks.candidates(samples, now().map_err(|_| Error::Lease)?),
            None => {
                worker.take();
                tracks.scan_error(now().map_err(|_| Error::Lease)?)
            }
        }
        .map_err(|_| Error::State)?;
        for value in candidates {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            let mut pending =
                live_relay_http::source::ObservationSubmission::new(publication, &value)
                    .map_err(Error::Http)?;
            let mut attempts = 0;
            loop {
                if stopped() {
                    return Ok(());
                }
                let control = observation
                    .lease
                    .control(client)
                    .map_err(|_| Error::Lease)?;
                if std::time::Instant::now() >= deadline {
                    return Err(Error::State);
                }
                let delay = std::time::Duration::from_millis(500);
                let accepted = match control.publish_source_observation(publication, &mut pending) {
                    Ok(v) => v,
                    Err(_) if stopped() => return Ok(()),
                    Err(error) if error.retryable_read() && attempts < 4 => {
                        let backoff =
                            std::time::Duration::from_millis((400u64 << attempts).min(2000));
                        attempts += 1;
                        signal.service_recovery(&signal.observation_recovering, true);
                        if !observation.lease.wait(
                            backoff
                                .min(deadline.saturating_duration_since(std::time::Instant::now())),
                        ) {
                            return Ok(());
                        }
                        continue;
                    }
                    Err(error) => return Err(Error::Http(error)),
                };
                if stopped() {
                    return Ok(());
                }
                if accepted {
                    signal.service_recovery(&signal.observation_recovering, false);
                    tracks.commit(value);
                    break;
                }
                attempts = 0;
                signal.service_recovery(&signal.observation_recovering, true);
                if std::time::Instant::now() >= deadline {
                    return Err(Error::State);
                }
                observation
                    .lease
                    .wait(delay.min(deadline.saturating_duration_since(std::time::Instant::now())));
            }
        }
        observation
            .lease
            .wait(std::time::Duration::from_millis(if worker.is_some() {
                500
            } else {
                1000
            }));
    }
    Ok(())
}
