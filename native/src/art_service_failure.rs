//! Transient service failures reject only that Promise; cleanup consumes the original Call without replay, offline eligibility, or authorization extension.
use super::*;

fn recoverable(error: Error) -> Option<&'static str> {
    use crate::{device_session_client::Error as Network, live_relay_http::Error as Http};
    match error {
        Error::Http(Http::Status(429)) => Some("service_busy"),
        Error::Http(Http::Status(502..=504)) => Some("service_unavailable"),
        // Client's finer Transport errors retain original Http disconnection semantics; reject only the current Promise, still without replaying submit.
        Error::Http(Http::Http(Network::Connect | Network::Transport | Network::Http)) => {
            Some("service_transport")
        }
        // TLS trust, quota, identity/sequence/wire, and authentication errors never justify continued local execution.
        _ => None,
    }
}
impl Dispatcher {
    fn continuation(&self, deadline: Instant, stopped: &dyn Fn() -> bool) -> io::Result<()> {
        if stopped() || self.lease.execution_stopped() || Instant::now() >= deadline {
            return Err(failure(Error::Stopped));
        }
        let time = now().map_err(|error| failure(Error::Http(error)))?;
        self.lease
            .validate(self.client.origin(), time)
            .map_err(|_| failure(Error::Lease))?;
        self.permit
            .lock()
            .map_err(|_| failure(Error::Stopped))?
            .authorize(&self.binding, time, true, self.lease.is_revoked())
            .map_err(|error| failure(Error::Grant(error)))?;
        Ok(())
    }
    pub(super) fn settle_failure(
        &self,
        call: Call<'_>,
        error: Error,
        deadline: Instant,
        stopped: &dyn Fn() -> bool,
    ) -> io::Result<services::ResultValue> {
        if self.lease.execution_stopped() || stopped() {
            // submit may have executed without Accepted arriving; authentication revocation still forbids any cleanup networking.
            let _ = call.abandon(deadline);
            return Err(failure(error));
        }
        let Some(code) = recoverable(error) else {
            return Err(failure(error));
        };
        self.continuation(deadline, stopped)?;
        if let Err(cleanup) = call.abandon(deadline) {
            // 404 only means the original call is absent, not success. Cleanup transport failure still relies on the service's original expiry for reclamation.
            if recoverable(cleanup).is_none()
                && !matches!(
                    cleanup,
                    Error::Time
                        | Error::Http(crate::live_relay_http::Error::Status(404))
                        | Error::Http(crate::live_relay_http::Error::Http(
                            crate::device_session_client::Error::Deadline
                        ))
                )
            {
                return Err(failure(cleanup));
            }
        }
        self.continuation(deadline, stopped)?;
        Ok(services::ResultValue::Failed { error: code.into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_transient_errors_are_call_local() {
        use crate::{device_session_client::Error as Network, live_relay_http::Error as Http};
        for status in [429, 502, 503, 504] {
            assert!(recoverable(Error::Http(Http::Status(status))).is_some());
        }
        for error in [Network::Connect, Network::Transport, Network::Http] {
            assert_eq!(
                recoverable(Error::Http(Http::Http(error))),
                Some("service_transport")
            );
        }
        for error in [
            Error::Input,
            Error::Limit,
            Error::Wire,
            Error::Identity,
            Error::Sequence,
            Error::Stopped,
            Error::Time,
            Error::Lease,
            Error::Grant(crate::art_execution_grant::Error::Time),
            Error::Http(Http::AuthorizationRequired),
            Error::Http(Http::Status(403)),
            Error::Http(Http::Status(500)),
            Error::Http(Http::Http(Network::Trust)),
            Error::Http(Http::Http(Network::Tls)),
            Error::Http(Http::Http(Network::Limit)),
            Error::Http(Http::Http(Network::Deadline)),
            Error::Http(Http::Http(Network::Stopped)),
        ] {
            assert_eq!(recoverable(error), None, "{error:?}");
        }
    }
}
