//! 图片 child 的固定入口及原有 128/256 MiB 策略；进程/Job 寿命归共用 owner。
pub(crate) use crate::bounded_worker_process::Child;
use std::{
    io,
    os::windows::io::OwnedHandle,
    sync::{Arc, LazyLock},
    time::Instant,
};
const CHILD_LIMIT: usize = 128 * 1024 * 1024;
#[cfg(test)]
#[path = "surface_webp_process_tests.rs"]
mod tests;
static CAPACITY: LazyLock<Arc<crate::byte_budget::Budget>> =
    LazyLock::new(|| crate::byte_budget::Budget::new(256 * 1024 * 1024));
#[cfg(test)]
fn capacity(
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<crate::byte_budget::Lease> {
    crate::bounded_worker_process::capacity(&CAPACITY, CHILD_LIMIT, deadline, stopped)
}
pub(super) fn spawn(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<Child> {
    let suffix = if cfg!(test) {
        " --exact surface_webp_worker::tests::webp_worker_entry --ignored --nocapture"
    } else {
        " --native-webp-worker"
    };
    start(endpoint, deadline, stopped, suffix)
}
pub(super) fn spawn_svg(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<Child> {
    let suffix = if cfg!(test) {
        " --exact surface_svg_worker::tests::svg_worker_entry --ignored --nocapture"
    } else {
        " --native-svg-worker"
    };
    start(endpoint, deadline, stopped, suffix)
}
pub(super) fn start(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
    suffix: &str,
) -> io::Result<Child> {
    crate::bounded_worker_process::start(
        endpoint,
        deadline,
        stopped,
        crate::bounded_worker_process::Policy {
            suffix,
            maximum: CHILD_LIMIT,
            pool: &CAPACITY,
            environment: crate::bounded_worker_process::Environment::Empty,
        },
    )
}
