//! Single-workflow request exchange in the background; callers own connection admission, receipt revalidation, and successful-receipt consumption.
use super::Request;
use crate::{byte_budget::Budget, loom_bridge_client::BridgeSocket};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::{
    borrow::Cow,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tungstenite::Message;
const RESPONSE_LIMIT: usize = 1024 * 1024;

#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Budget,
    Send,
    Closed,
    Timeout,
    InvalidResponse,
    Rejected,
}

/// Only keep the fields used for reply routing. The rest still passes through
/// the JSON parser (including number, Unicode and nesting checks), without
/// allocating a Value tree for an unrelated event or unused result data.
#[derive(Default)]
struct Ack<'a> {
    protocol: Option<Cow<'a, str>>,
    request: Option<Cow<'a, str>>,
    status: Option<Cow<'a, str>>,
    method: bool,
    workflow: Option<Cow<'a, str>>,
}
#[derive(serde::Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum Key {
    ProtocolVersion,
    RequestId,
    Status,
    Method,
    Data,
    WorkflowId,
    #[serde(other)]
    Other,
}
#[derive(Clone, Copy)]
enum Scan {
    Envelope,
    Data,
    Text,
    Skip,
}
enum Parsed<'a> {
    Ack(Ack<'a>),
    Text(Cow<'a, str>),
    Null,
    Other,
}
impl<'a> Parsed<'a> {
    fn text(self) -> Option<Cow<'a, str>> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }
}
impl<'de> DeserializeSeed<'de> for Scan {
    type Value = Parsed<'de>;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Scan {
    type Value = Parsed<'de>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON workflow reply")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut reply = Ack::default();
        while let Some(key) = map.next_key::<Key>()? {
            // Like serde_json::Value, use the last occurrence, even if it has a
            // different type. Malformed JSON never yields a partial reply.
            match (self, key) {
                (Self::Envelope, Key::ProtocolVersion) => {
                    reply.protocol = map.next_value_seed(Self::Text)?.text()
                }
                (Self::Envelope, Key::RequestId) => {
                    reply.request = map.next_value_seed(Self::Text)?.text()
                }
                (Self::Envelope, Key::Status) => {
                    reply.status = map.next_value_seed(Self::Text)?.text()
                }
                (Self::Envelope, Key::Method) => {
                    reply.method = !matches!(map.next_value_seed(Self::Skip)?, Parsed::Null)
                }
                (Self::Envelope, Key::Data) => {
                    reply.workflow = map.next_value_seed(Self::Data)?.text()
                }
                (Self::Data, Key::WorkflowId) => {
                    reply.workflow = map.next_value_seed(Self::Text)?.text()
                }
                _ => {
                    map.next_value_seed(Self::Skip)?;
                }
            }
        }
        Ok(match self {
            Self::Envelope => Parsed::Ack(reply),
            Self::Data => reply.workflow.map_or(Parsed::Other, Parsed::Text),
            _ => Parsed::Other,
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut array: A) -> Result<Self::Value, A::Error> {
        while array.next_element_seed(Self::Skip)?.is_some() {}
        Ok(Parsed::Other)
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(if matches!(self, Self::Text) {
            Parsed::Text(Cow::Borrowed(value))
        } else {
            Parsed::Other
        })
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(if matches!(self, Self::Text) {
            Parsed::Text(Cow::Owned(value.to_owned()))
        } else {
            Parsed::Other
        })
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(Parsed::Other)
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(Parsed::Other)
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(Parsed::Other)
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(Parsed::Other)
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(Parsed::Null)
    }
}
fn parse_reply(bytes: &[u8]) -> Option<Ack<'_>> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let Parsed::Ack(reply) = Scan::Envelope.deserialize(&mut decoder).ok()? else {
        return None;
    };
    decoder.end().ok()?;
    Some(reply)
}

/// Return no arbitrary remote error text, preventing credentials/paths from echoing through logs or UI.
pub fn response(bytes: &[u8], request: &Request) -> Result<bool, Error> {
    if bytes.len() > RESPONSE_LIMIT {
        return Err(Error::InvalidResponse);
    }
    let Some(reply) = parse_reply(bytes) else {
        return Ok(false);
    };
    if reply.protocol.as_deref() != Some("loom.hook.v1")
        || reply.request.as_deref() != Some(request.id())
        || reply.method
    {
        return Ok(false);
    }
    match reply.status.as_deref() {
        Some("succeeded") if reply.workflow.as_deref() == Some(request.workflow_id()) => Ok(true),
        Some("failed" | "cancelled") => Err(Error::Rejected),
        _ => Err(Error::InvalidResponse),
    }
}

/// One attempt with a five-second total deadline; timeout neither resends automatically nor advances batches or changes local sources.
pub fn run(
    socket: BridgeSocket,
    request: &Request,
    budget: &Arc<Budget>,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    check(cancel)?;
    // Tungstenite owns Text; independently admit its send copy rather than double-accounting against the batch lease.
    let mut lease = budget.reserve(request.bytes().len()).ok_or(Error::Budget)?;
    let mut message = String::new();
    message
        .try_reserve_exact(request.bytes().len())
        .map_err(|_| Error::Budget)?;
    if !lease.grow_to(message.capacity()) {
        return Err(Error::Budget);
    }
    message.push_str(std::str::from_utf8(request.bytes()).map_err(|_| Error::InvalidResponse)?);
    // Inner scope destroys the socket, including possibly unsent Text, before outer scope returns the lease.
    exchange(socket, request, message, cancel)
}
fn exchange(
    mut socket: BridgeSocket,
    request: &Request,
    message: String,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    socket.set_config(|config| {
        config.max_message_size = Some(RESPONSE_LIMIT);
        config.max_frame_size = Some(RESPONSE_LIMIT);
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    set_deadline(&mut socket, deadline, Duration::from_secs(5))?;
    check(cancel)?;
    socket
        .send(Message::Text(message))
        .map_err(|_| Error::Send)?;
    loop {
        check(cancel)?;
        set_deadline(&mut socket, deadline, Duration::from_millis(250))?;
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                continue
            }
            Err(_) => {
                return Err(if Instant::now() >= deadline {
                    Error::Timeout
                } else {
                    Error::Closed
                })
            }
        };
        let reply = match &message {
            Message::Text(text) => response(text.as_bytes(), request),
            _ => Ok(false),
        };
        if settle_received(reply, cancel)? {
            return Ok(());
        }
        match message {
            Message::Ping(_) => {
                set_deadline(&mut socket, deadline, Duration::from_millis(250))?;
                socket.flush().map_err(|_| Error::Closed)?;
            }
            Message::Close(_) => return Err(Error::Closed),
            _ => {}
        }
    }
}
fn settle_received(reply: Result<bool, Error>, cancel: &AtomicBool) -> Result<bool, Error> {
    // A matching terminal reply already read from the socket remains evidence
    // even if cancellation arrived during that read. Do not wait for another.
    if !matches!(reply, Ok(true) | Err(Error::Rejected)) {
        check(cancel)?;
    }
    reply
}

fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn set_deadline(socket: &mut BridgeSocket, deadline: Instant, cap: Duration) -> Result<(), Error> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(remaining.min(cap))
        .map_err(|_| Error::Closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_reply_already_read_is_not_erased_by_cancellation() {
        // This models cancellation after a frame was read, not permission
        // to start or continue socket I/O after cancellation.
        let cancelled = AtomicBool::new(true);
        assert_eq!(settle_received(Ok(true), &cancelled), Ok(true));
        assert_eq!(
            settle_received(Err(Error::Rejected), &cancelled),
            Err(Error::Rejected)
        );
        assert_eq!(
            settle_received(Ok(false), &cancelled),
            Err(Error::Cancelled)
        );
        assert_eq!(
            settle_received(Err(Error::InvalidResponse), &cancelled),
            Err(Error::Cancelled)
        );
    }
}
