//! Background catalog discovery shares the four-task viewing limit; completed/unreaped tasks retain slots, and UI never loads CA/DNS.
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{CatalogEntry, Error as HttpError},
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::Arc,
    thread::JoinHandle,
};

pub struct Catalog {
    pub origin: String,
    pub device_id: String,
    pub entries: Vec<CatalogEntry>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Http(HttpError),
}
enum Connection {
    Configured(Config),
    Prepared(Client),
}
pub struct Job {
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Result<Catalog, Error>>>,
    slot: Option<Slot>,
}
impl Job {
    pub fn start(config: Config, lease: Lease) -> io::Result<Self> {
        Self::spawn(Connection::Configured(config), lease)
    }
    pub fn start_prepared(client: Client, lease: Lease) -> io::Result<Self> {
        Self::spawn(Connection::Prepared(client), lease)
    }
    fn spawn(connection: Connection, lease: Lease) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-live-catalog".into())
            .spawn(move || {
                if task.is_revoked() {
                    return Err(Error::Stopped);
                }
                let client = match connection {
                    Connection::Prepared(client) => client,
                    Connection::Configured(config) => {
                        task.prepare_client(&config).map_err(Error::Preparation)?
                    }
                };
                let control = task.control(&client).map_err(|_| Error::Stopped)?;
                let entries = control.catalog().map_err(Error::Http)?;
                if task.is_revoked() {
                    return Err(Error::Stopped);
                }
                Ok(Catalog {
                    origin: client.origin().to_owned(),
                    device_id: task.device_id().to_owned(),
                    entries,
                })
            })?;
        Ok(Self {
            lease,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Result<Catalog, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Err(Error::Panicked));
        self.slot.take();
        // Even completed successful catalogs cannot bypass service switches/revocation; consumers need a fresh lease and query.
        Some(if self.lease.is_revoked() {
            Err(Error::Stopped)
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // Normal paths must wait/try_join; no detach here, only a blocking leak-prevention fallback.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
