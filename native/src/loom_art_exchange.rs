//! One-shot background Art message exchange; no threads/queues, with callers immediately taking preview-resource lifetime ownership.
use crate::{
    graph_requests::Request,
    loom_art_response::{self, Event},
    loom_bridge_client::BridgeSocket,
    workspace_id::UnitId,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tungstenite::Message;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidTimeout,
    Send,
    Closed,
    Timeout,
    Response(loom_art_response::Error),
    Cancelled,
}

/// Input-resource owners must survive this call. on_update returning false terminates exchange and closes the local connection.
/// Callers must filter UI updates with complete request tokens and own the preview/result resource-release protocol.
pub fn run(
    socket: BridgeSocket,
    message: String,
    unit: &UnitId,
    request: &Request,
    timeout: Duration,
    on_update: impl FnMut(Event) -> bool,
) -> Result<Event, Error> {
    run_cancellable(socket, message, unit, request, timeout, None, on_update)
}
/// Cancelled tasks recheck flags with a 250 ms read budget, avoiding sole reliance on Windows shutdown wake races.
pub fn run_cancellable(
    mut socket: BridgeSocket,
    message: String,
    unit: &UnitId,
    request: &Request,
    timeout: Duration,
    cancel: Option<&AtomicBool>,
    mut on_update: impl FnMut(Event) -> bool,
) -> Result<Event, Error> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .filter(|_| !timeout.is_zero())
        .ok_or(Error::InvalidTimeout)?;
    set_deadline(&mut socket, deadline, Some(Duration::from_secs(5)))?;
    socket
        .send(Message::Text(message))
        .map_err(|_| Error::Send)?;
    loop {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
            return Err(Error::Cancelled);
        }
        set_deadline(
            &mut socket,
            deadline,
            cancel.map(|_| Duration::from_millis(250)),
        )?;
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if cancel.is_some()
                    && matches!(
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
        match message {
            Message::Text(text) => match loom_art_response::parse(text.as_bytes(), unit, request)
                .map_err(Error::Response)?
            {
                Event::Ignore => {}
                event @ (Event::Result { .. } | Event::Failure(_)) => return Ok(event),
                update => {
                    if !on_update(update) {
                        return Err(Error::Cancelled);
                    }
                }
            },
            Message::Ping(_) => {
                // Tungstenite already queued Pong; perform a separately bounded flush without heartbeat extension of the total response deadline.
                set_deadline(&mut socket, deadline, Some(Duration::from_secs(5)))?;
                socket.flush().map_err(|_| Error::Closed)?;
            }
            Message::Close(_) => return Err(Error::Closed),
            _ => {}
        }
    }
}

fn set_deadline(
    socket: &mut BridgeSocket,
    deadline: Instant,
    cap: Option<Duration>,
) -> Result<(), Error> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(cap.map_or(remaining, |cap| remaining.min(cap)))
        .map_err(|_| Error::Closed)
}
