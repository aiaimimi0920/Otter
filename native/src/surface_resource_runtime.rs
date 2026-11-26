//! Bounded image-fetch tasks; ready bytes still occupy shared task slots and retain original leases until consumption/discard.
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{MountedSurface, ResourceError, ResourceLease},
    surface_live_binding::Binding,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::Arc,
    thread::JoinHandle,
};
#[path = "surface_resource_decode.rs"]
mod decode;
use decode::Decoded;
#[path = "surface_image_request.rs"]
pub mod request;
use request::{Key, Request};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub binding: Binding,
    pub revision: u64,
    pub key: Key,
    pub expires_at_ms: u64,
}
impl Target {
    fn from_request(request: &Request) -> Self {
        Self {
            binding: request.binding().clone(),
            revision: request.revision(),
            key: request.key(),
            expires_at_ms: request.expires_at_ms(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Resource(ResourceError),
    Decode(i32),
}
enum Connection {
    Configured(Config),
    Prepared(Client),
    Inline,
}
pub struct Ready {
    resource: Decoded,
    lease: Arc<Lease>,
    // Completion does not release memory; unconsumed pixels still occupy the four global slots and separately obey shared Pixels budgets.
    _slot: Slot,
}
impl Ready {
    /// After successful consumption, a window lease with the same Grant takes over, releasing request slots/leases without accumulating network owners.
    /// This compatibility interface returns only rasters; Surface UI installation must use transfer_image to retain logical dimensions.
    pub fn transfer(
        self,
        destination: &Lease,
        current: &MountedSurface,
        now: u64,
    ) -> Result<(ResourceLease, crate::image_decode::Image), Error> {
        let (request, image, _) = self.transfer_image(destination, current, now)?;
        match request {
            Request::Resource(request) => Ok((request, image)),
            Request::Inline(_) => Err(Error::Resource(ResourceError::Unsupported)),
        }
    }
    pub(crate) fn transfer_image(
        self,
        destination: &Lease,
        current: &MountedSurface,
        now: u64,
    ) -> Result<(Request, crate::image_decode::Image, [f32; 2]), Error> {
        self.image(current, now)?;
        if !self.lease.same_grant(destination) {
            return Err(Error::Resource(ResourceError::Identity));
        }
        self.resource
            .request
            .validate(destination, current, now)
            .map_err(Error::Resource)?;
        Ok((
            self.resource.request,
            self.resource.image,
            self.resource.logical_size,
        ))
    }
    #[cfg(test)]
    pub(crate) fn from_test_image(
        lease: Lease,
        request: ResourceLease,
        image: crate::image_decode::Image,
    ) -> Self {
        Self {
            resource: Decoded {
                request: Request::Resource(request),
                logical_size: [image.width as f32, image.height as f32],
                image,
            },
            lease: Arc::new(lease),
            _slot: Slot::acquire().unwrap(),
        }
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.resource
            .request
            .expires_at_ms()
            .min(self.lease.expires_at_ms())
    }
    pub fn binding(&self) -> &Binding {
        self.resource.request.binding()
    }
    pub fn revision(&self) -> u64 {
        self.resource.request.revision()
    }
    pub fn key(&self) -> Key {
        self.resource.request.key()
    }
    pub fn mime(&self) -> &str {
        self.resource.request.mime()
    }
    pub fn image<'a>(
        &'a self,
        current: &MountedSurface,
        now: u64,
    ) -> Result<&'a crate::image_decode::Image, Error> {
        if self.lease.is_revoked() {
            return Err(Error::Stopped);
        }
        self.resource
            .request
            .validate(&self.lease, current, now)
            .map_err(Error::Resource)?;
        Ok(&self.resource.image)
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
}
pub struct Job {
    target: Target,
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Result<Decoded, Error>>>,
    slot: Option<Slot>,
}
impl Job {
    pub fn start(config: Config, lease: Lease, request: ResourceLease) -> io::Result<Self> {
        Self::spawn(
            Connection::Configured(config),
            lease,
            Request::Resource(request),
        )
    }
    pub fn start_prepared(
        client: Client,
        lease: Lease,
        request: ResourceLease,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Prepared(client),
            lease,
            Request::Resource(request),
        )
    }
    pub fn start_inline(lease: Lease, request: request::inline::Inline) -> io::Result<Self> {
        Self::spawn(Connection::Inline, lease, Request::Inline(request))
    }
    fn spawn(connection: Connection, lease: Lease, request: Request) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let target = Target::from_request(&request);
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-resource".into())
            .spawn(move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if task.is_revoked() {
                        return Err(Error::Stopped);
                    }
                    let request = match request {
                        Request::Inline(request) => return decode::inline(request, &task),
                        Request::Resource(request) => request,
                    };
                    let client = match connection {
                        Connection::Inline => return Err(Error::Stopped),
                        Connection::Prepared(client) => client,
                        Connection::Configured(config) => {
                            // Trust files and resolved addresses may change at the same configured path.
                            task.prepare_client(&config).map_err(Error::Preparation)?
                        }
                    };
                    let resource = task
                        .control(&client)
                        .map_err(|_| Error::Stopped)?
                        .fetch_image_resource(request)
                        .map_err(Error::Resource)?;
                    decode::decode(resource, &task)
                }))
                .unwrap_or(Err(Error::Panicked))
            })?;
        Ok(Self {
            target,
            lease,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn target(&self) -> &Target {
        &self.target
    }
    pub fn deadline_ms(&self) -> Option<u64> {
        (!self.lease.is_revoked())
            .then_some(self.target.expires_at_ms.min(self.lease.expires_at_ms()))
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|t| t.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Result<Ready, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Err(Error::Panicked));
        let slot = self.slot.take().expect("completed task retains capacity");
        if self.lease.is_revoked() {
            return Some(Err(Error::Stopped));
        }
        Some(result.map(|resource| Ready {
            resource,
            lease: self.lease.clone(),
            _slot: slot,
        }))
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // Successful join transfers leases/slots to Ready; reaping Job must not revoke the new owner's valid result.
        if let Some(thread) = self.thread.take() {
            self.request_stop();
            let _ = thread.join();
        }
    }
}
