//! Aggregate publishing/control/observation exit reasons; received terminal authorization states outrank ordinary failure.
use super::*;

impl Exit {
    pub(super) fn authorization_ended(self) -> bool {
        use live_relay_http::Error::AuthorizationRequired;
        matches!(
            self,
            Self::Revoked
                | Self::Http(AuthorizationRequired)
                | Self::Control(control::Error::Http(AuthorizationRequired))
                | Self::Observation(observation::Error::Http(AuthorizationRequired))
        )
    }
}

pub(super) fn source_exit(
    requested: bool,
    source: Result<(), Exit>,
    control: Option<Result<(), control::Error>>,
    observation: Option<Result<(), observation::Error>>,
) -> Exit {
    let control = control.and_then(Result::err).map(Exit::Control);
    let observation = observation.and_then(Result::err).map(Exit::Observation);
    let source = source.err();
    // Cleanup permission depends on actual terminal authorization states received by all three chains, not ordinary-error display order.
    [control, observation, source]
        .into_iter()
        .flatten()
        .find(|exit| exit.authorization_ended())
        .or(control)
        .or(observation)
        .unwrap_or_else(|| {
            if requested {
                Exit::Stopped
            } else {
                source.unwrap_or(Exit::Stopped)
            }
        })
}
