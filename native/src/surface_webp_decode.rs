//! 父侧 WebP runner：有限协议、共享 Pixels 租约、截止时间和撤销；没有主进程解码回退。
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
// 启动接缝供测试故障 child；生产调用只绑定当前 EXE 的固定私有入口。
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
