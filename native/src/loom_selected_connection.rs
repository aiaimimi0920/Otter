//! Explicit-address connections reuse saved trust; initial LAN discovery pins that address's certificate before normal TLS verification.
use crate::{
    device_request_cancel::Cancel,
    loom_local_connection::{Connected, Error},
    loom_remote_connection::{Document, Verified},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    net::{IpAddr, UdpSocket},
    path::Path,
    time::{Duration, Instant},
};
pub(crate) fn connect(
    origin: &str,
    profile: &Path,
    cancel: &Cancel,
    password: &str,
) -> Result<Connected, Error> {
    if let Some(remote) = Document::load(profile)? {
        if remote.origin == origin {
            return remote.connect_with_password(profile, cancel, password);
        }
    }
    // Local manifests already provide trust; old remote configuration cannot override the user's newly selected local address.
    if let Some(result) =
        crate::loom_local_connection::connect_selected_local(origin, profile, cancel)
    {
        return result;
    }
    let url = url::Url::parse(origin).map_err(|_| Error::Response)?;
    let ip = url
        .host_str()
        .and_then(|h| h.parse::<std::net::Ipv4Addr>().ok());
    let verified = if let Some(ip) = ip.filter(|ip| ip.is_private() || ip.is_loopback()) {
        discover(origin, IpAddr::V4(ip), cancel)?.verify(cancel)?
    } else {
        Verified::public(origin, cancel)?
    };
    verified.connect_new(profile, cancel, password)
}
fn discover(origin: &str, ip: IpAddr, cancel: &Cancel) -> Result<Document, Error> {
    let socket = UdpSocket::bind("0.0.0.0:49779").map_err(|_| Error::Discovery)?;
    socket
        .set_read_timeout(Some(Duration::from_millis(150)))
        .map_err(|_| Error::Discovery)?;
    let until = Instant::now() + Duration::from_secs(4);
    let mut packet = [0; 8193];
    while Instant::now() < until && !cancel.is_stopped() {
        let (size, peer) = match socket.recv_from(&mut packet) {
            Ok(value) => value,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(_) => return Err(Error::Discovery),
        };
        if peer.ip() != ip || size > 8192 {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&packet[..size]) else {
            continue;
        };
        if value["endpoint"] != origin || value["protocolVersion"] != "loom.connection.v1" {
            continue;
        }
        let cert = value["certificate"].as_str().ok_or(Error::Tls)?;
        let bytes = STANDARD.decode(cert).map_err(|_| Error::Tls)?;
        let certificate_pem = format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            STANDARD.encode(bytes)
        );
        return Ok(Document {
            schema_version: 1,
            origin: origin.into(),
            device_id: None,
            loom_id: value["loomId"]
                .as_str()
                .filter(|s| crate::projection_delivery::share::loom_id(s))
                .ok_or(Error::Response)?
                .into(),
            certificate_pem,
        });
    }
    Err(if cancel.is_stopped() {
        Error::Stopped
    } else {
        Error::Discovery
    })
}
pub(crate) fn save(profile: &Path, value: &Document, cancel: &Cancel) -> Result<(), Error> {
    let _pin = crate::profile_directory::pin(profile).map_err(|_| Error::Store)?;
    let path = profile.join("loom-remote.json");
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Store)?;
    let reserve = crate::pixels::reserve(bytes.capacity()).ok_or(Error::Store)?;
    let bytes = crate::pixels::Pixels::from_reserved(bytes, reserve);
    match Document::read(profile)? {
        Some(before) => {
            crate::session_file_replace::Replacement::prepare_bytes(
                &path,
                &before,
                bytes,
                cancel.flag(),
            )
            .map_err(|_| Error::Store)?
            .commit(cancel.flag())
            .map_err(|_| Error::Store)?;
        }
        None => {
            crate::session_file_write::Staged::prepare_bytes(&path, bytes, cancel.flag())
                .map_err(|_| Error::Store)?
                .commit_new(cancel.flag())
                .map_err(|_| Error::Store)?;
        }
    }
    Ok(())
}
