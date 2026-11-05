//! 仅保留可辨认的传输损失；协议/TLS验证/资源超限不能伪装成临时网错。这里不自动重试。
use super::Error;
use std::io;
pub(super) fn transient_io(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::NotConnected
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::TimedOut
            | io::ErrorKind::WouldBlock
    )
}
pub(super) fn http(error: &ureq::Error) -> Error {
    match error {
        ureq::Error::Io(error) if transient_io(error) => Error::Transport,
        ureq::Error::Timeout(_) => Error::Deadline,
        _ => Error::Http,
    }
}
pub(super) fn body(error: &io::Error) -> Error {
    if let Some(error) = error
        .get_ref()
        .and_then(|e| e.downcast_ref::<ureq::Error>())
    {
        return http(error);
    }
    if transient_io(error) {
        Error::Transport
    } else {
        Error::Http
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn io_transport_is_distinct_from_protocol_limits_and_tls() {
        for kind in [
            io::ErrorKind::UnexpectedEof,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::TimedOut,
        ] {
            assert_eq!(
                http(&ureq::Error::Io(io::Error::from(kind))),
                Error::Transport
            );
            assert_eq!(body(&io::Error::from(kind)), Error::Transport);
        }
        for kind in [
            io::ErrorKind::InvalidData,
            io::ErrorKind::InvalidInput,
            io::ErrorKind::PermissionDenied,
        ] {
            assert_eq!(body(&io::Error::from(kind)), Error::Http);
        }
        assert_eq!(
            http(&ureq::Error::LargeResponseHeader(20000, 16384)),
            Error::Http
        );
        assert_eq!(
            body(&io::Error::other(ureq::Error::Tls("invalid certificate"))),
            Error::Http
        );
    }
}
