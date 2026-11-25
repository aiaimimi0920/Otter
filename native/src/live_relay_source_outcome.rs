//! 汇总发布、控制和观察的退出原因；已收到的授权终态优先于普通失败。
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
    // 清理权限取决于三条链实际收到的授权终态，不由普通错误的展示顺序决定。
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
