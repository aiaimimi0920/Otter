//! Parent-side WebP runner: bounded protocol, shared Pixels leases, deadlines, and revocation; no main-process decoding fallback.
use super::*;
use crate::surface_webp_header::inspect;
#[cfg(test)]
use crate::{live_recovery_pipe::Pipe, surface_webp_header::invalid};
use crate::{surface_image_worker_process as process, surface_image_worker_wire as wire};
use std::{io, time::Instant};
#[cfg(test)]
use std::{
    io::{Read, Write},
    time::Duration,
};
#[cfg(test)]
#[path = "surface_webp_wire_tests.rs"]
mod wire_tests;
pub(super) fn decode(bytes: &[u8], stopped: &impl Fn() -> bool) -> Result<Image> {
    run(bytes, stopped).map_err(|e| Error::new(E_INVALIDARG, format!("Surface WebP decode: {e}")))
}
fn run(bytes: &[u8], stopped: &impl Fn() -> bool) -> io::Result<Image> {
    run_with(bytes, stopped, |endpoint, deadline| {
        process::spawn(endpoint, deadline, stopped)
    })
}
// Launch seam for faulty test children; production calls bind only the current EXE's fixed private entry.
fn run_with(
    bytes: &[u8],
    stopped: &impl Fn() -> bool,
    spawn: impl FnOnce(std::os::windows::io::OwnedHandle, Instant) -> io::Result<process::Child>,
) -> io::Result<Image> {
    let (width, height) = inspect(bytes)?;
    worker_client::run(
        bytes,
        wire::Codec::WebP,
        Some((width, height)),
        stopped,
        spawn,
    )
}
