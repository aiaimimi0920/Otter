//! UIA私有MTA进程；复用唯一Job/pipe owner，单在途扫描2秒、128MiB/全进程512MiB。
use super::*;
use crate::{
    bounded_worker_process as bounded, live_recovery_pipe::Pipe, live_window_target::Target,
};
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    os::windows::io::AsHandle,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
pub const FLAG: &str = "--native-observation-worker";
const LIMIT: usize = 128 * 1024 * 1024;
const MAX_WIRE: usize = 512 * 1024;
static CAPACITY: LazyLock<Arc<crate::byte_budget::Budget>> =
    LazyLock::new(|| crate::byte_budget::Budget::new(4 * LIMIT));
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid UIA worker data")
}
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|a| a == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 || args[0] != FLAG {
        return Err(invalid());
    }
    Ok(true)
}
pub(crate) struct Worker {
    pipe: Pipe,
    child: bounded::Child,
    target: Target,
}
impl Worker {
    pub(crate) fn start(target: Target, stopped: &impl Fn() -> bool) -> io::Result<Self> {
        target.validate().map_err(io::Error::other)?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let (mut pipe, endpoint) = Pipe::pair()?;
        let child = bounded::start(
            endpoint.into_handle(),
            deadline,
            stopped,
            bounded::Policy {
                suffix: if cfg!(test) {
                    " --exact live_source_uia::process::tests::worker_entry --ignored --nocapture"
                } else {
                    " --native-observation-worker"
                },
                maximum: LIMIT,
                pool: &CAPACITY,
                environment: bounded::Environment::SystemRoot,
            },
        )?;
        let mut channel = pipe.cancellable_io(deadline, child.handle(), stopped);
        channel.write_all(b"OUI1")?;
        channel.write_all(&(target.hwnd().0 as usize as u64).to_le_bytes())?;
        channel.write_all(&target.process_id().to_le_bytes())?;
        target.validate().map_err(io::Error::other)?;
        Ok(Self {
            pipe,
            child,
            target,
        })
    }
    pub(crate) fn scan(&mut self, stopped: &impl Fn() -> bool) -> io::Result<Vec<Sample>> {
        self.target.validate().map_err(io::Error::other)?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut channel = self
            .pipe
            .cancellable_io(deadline, self.child.handle(), stopped);
        channel.write_all(b"S")?;
        let bytes = read_frame(&mut channel)?;
        let samples: Vec<Sample> = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        if samples.len() > 256 {
            return Err(invalid());
        }
        self.target.validate().map_err(io::Error::other)?;
        Ok(samples)
    }
}
fn read_frame(reader: &mut impl Read) -> io::Result<zeroize::Zeroizing<Vec<u8>>> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_WIRE {
        return Err(invalid());
    }
    let mut bytes = zeroize::Zeroizing::new(vec![0; length]);
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}
pub fn run_inherited() -> io::Result<()> {
    bounded::require_limits(LIMIT)?;
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(2)), None);
    let mut header = [0; 16];
    channel.read_exact(&mut header)?;
    if &header[..4] != b"OUI1" {
        return Err(invalid());
    }
    let hwnd = HWND(
        usize::try_from(u64::from_le_bytes(header[4..12].try_into().unwrap()))
            .map_err(|_| invalid())? as *mut _,
    );
    let target = Target::bind(hwnd).map_err(io::Error::other)?;
    if target.process_id() != u32::from_le_bytes(header[12..].try_into().unwrap()) {
        return Err(invalid());
    }
    let scanner = Scanner::new().map_err(io::Error::other)?;
    loop {
        let mut command = [0];
        pipe.io(None, None).read_exact(&mut command)?;
        if command != *b"S" {
            return Err(invalid());
        }
        let samples = scanner
            .scan(&target)
            .unwrap_or_else(|_| vec![root_error("uia_scan_failed")]);
        let bytes = zeroize::Zeroizing::new(serde_json::to_vec(&samples).map_err(|_| invalid())?);
        if bytes.len() > MAX_WIRE {
            return Err(invalid());
        }
        let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(2)), None);
        channel.write_all(&(bytes.len() as u32).to_le_bytes())?;
        channel.write_all(&bytes)?;
    }
}
#[cfg(test)]
#[path = "live_source_uia_process_tests.rs"]
mod tests;
