//! Local Loom interface for native account projection; no central coordination/Iroh implementation, and network success is not saving/presentation.
use crate::{device_request_cancel::Cancel, device_session_attempt::Session};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
#[path = "projection_account_envelope.rs"]
mod envelope;
pub use envelope::Envelope;
#[path = "projection_account_http.rs"]
mod http;
#[path = "projection_account_operation.rs"]
mod operation;
pub use operation::Operation;
#[path = "projection_account_response.rs"]
mod response;
pub use response::{Content, State};
#[path = "projection_account_journal.rs"]
pub mod journal;
#[path = "projection_account_scene.rs"]
pub mod scene;
pub const PROTOCOL: &str = "neuro.qr-projection.v2";
const MAX_WIRE: usize = 6 * 1024 * 1024;
const MAX_REVISION: u64 = 9_007_199_254_740_991;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Discovery,
    LocalOnly,
    Credential,
    Identity,
    Input,
    Envelope,
    Response,
    Image,
    Budget,
    Stopped,
    Http,
    Remote { status: u16, code: Option<String> },
}
/// No account credentials; account deviceId and local Otter Device actor are distinct identities.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub status: String,
    pub protocol: String,
    pub projection_protocol: String,
    pub origin: String,
    pub account_id: String,
    pub device_id: String,
    pub device_name: String,
    pub expires_at_ms: u64,
    pub policy: serde_json::Value,
}
impl Context {
    fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 16 * 1024 {
            return Err(Error::Response);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| Error::Response)?;
        if value.status != "signed_in"
            || value.protocol != "neuro.loom-account.v1"
            || value.projection_protocol != PROTOCOL
            || !envelope::origin(&value.origin)
            || !identifier(&value.account_id)
            || !envelope::device(&value.device_id)
            || value.device_name.len() > 1024
            || value.device_name.chars().any(char::is_control)
            || value.expires_at_ms <= now()?
            || value.expires_at_ms > MAX_REVISION
            || !value.policy.is_object()
            || value.policy["serverOrigin"] != value.origin
        {
            return Err(Error::Response);
        }
        Ok(value)
    }
}
/// Encoded/compressed images and WIC decoding share existing PNG/shared-pixel budgets without another codec implementation.
pub struct Image(crate::projection_delivery::sender::snapshot::Snapshot);
impl Image {
    pub fn from_png(
        png: &crate::pixels::Pixels,
        width: u32,
        height: u32,
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        crate::projection_delivery::sender::snapshot::Snapshot::from_png(png, width, height, cancel)
            .map(Self)
            .map_err(image_error)
    }
    pub fn width(&self) -> u32 {
        self.0.width
    }
    pub fn height(&self) -> u32 {
        self.0.height
    }
    pub fn digest(&self) -> &str {
        &self.0.digest
    }
    pub fn png(&self, cancel: &AtomicBool) -> Result<crate::pixels::Pixels, Error> {
        self.0.png(cancel).map_err(image_error)
    }
}
impl Serialize for Image {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
fn image_error(error: crate::projection_delivery::sender::Error) -> Error {
    use crate::projection_delivery::sender::Error as E;
    match error {
        E::Stopped => Error::Stopped,
        E::Limit => Error::Budget,
        _ => Error::Image,
    }
}
/// One serial original-lease chain; central origin only validates response identity, never constructs central HTTP requests.
pub struct Account<'a> {
    endpoint: http::Endpoint,
    session: &'a Session,
    cancel: &'a Cancel,
    context: Context,
}
impl<'a> Account<'a> {
    #[cfg(test)]
    pub(crate) fn open(
        target: crate::loom_media_target::Target,
        session: &'a Session,
        cancel: &'a Cancel,
    ) -> Result<Self, Error> {
        check(cancel)?;
        let endpoint = http::Endpoint::new(target)?;
        Self::with_endpoint(endpoint, session, cancel)
    }
    pub(crate) fn authorized(
        client: crate::device_session_client::Client,
        session: &'a Session,
        cancel: &'a Cancel,
    ) -> Result<Self, Error> {
        Self::with_endpoint(http::Endpoint::authorized(client), session, cancel)
    }
    fn with_endpoint(
        endpoint: http::Endpoint,
        session: &'a Session,
        cancel: &'a Cancel,
    ) -> Result<Self, Error> {
        let bytes = endpoint.request("GET", "/v1/otter/identity", &[], None, cancel, 16 * 1024)?;
        let identity: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| Error::Identity)?;
        if identity["protocolVersion"] != "loom.connection.v1"
            || (!endpoint.authenticates(session.origin())
                && identity["endpoint"] != session.origin())
            || !identity["loomId"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && s.len() <= 256)
        {
            return Err(Error::Identity);
        }
        let context = endpoint.request(
            "POST",
            "/v1/projections/v2/context",
            br#"{"kind":"context"}"#,
            Some(session),
            cancel,
            16 * 1024,
        )?;
        let context = Context::parse(&context)?;
        session
            .token_for(session.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        check(cancel)?;
        Ok(Self {
            endpoint,
            session,
            cancel,
            context,
        })
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn execute(&self, operation: Operation<'_>) -> Result<State, Error> {
        check(self.cancel)?;
        if now()? >= self.context.expires_at_ms {
            return Err(Error::Credential);
        }
        operation.validate(&self.context)?;
        // Original request/response each consume original shared budgets; deserialize borrowed PNG strings without copying whole JSON.
        let mut budget =
            crate::pixels::reserve(MAX_WIRE.next_power_of_two()).ok_or(Error::Budget)?;
        let body = operation.body()?;
        if body.len() > MAX_WIRE || !budget.grow_to(body.capacity()) {
            return Err(Error::Budget);
        }
        let path = format!("/v1/projections/v2/{}", operation.route());
        let bytes = self.endpoint.request(
            "POST",
            &path,
            &body,
            Some(self.session),
            self.cancel,
            MAX_WIRE,
        )?;
        let state = response::parse(&bytes, &operation, &self.context, self.cancel.flag())?;
        self.session
            .token_for(self.session.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        check(self.cancel)?;
        Ok(state)
    }
}
fn check(cancel: &Cancel) -> Result<(), Error> {
    if cancel.is_stopped() {
        Err(Error::Stopped)
    } else {
        Ok(())
    }
}
fn now() -> Result<u64, Error> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|n| u64::try_from(n.as_millis()).ok())
        .ok_or(Error::Credential)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn revision(value: u64) -> bool {
    (1..=MAX_REVISION).contains(&value)
}
#[cfg(test)]
#[path = "projection_account_tests.rs"]
pub(crate) mod tests;
