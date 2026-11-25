//! Fixed image-child entries and original 128/256 MiB policies; shared owners manage process/Job lifetimes.
pub use crate::bounded_worker_process::Child;
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
pub(crate) fn at_capacity() -> bool {
    CAPACITY.used() >= 256 * 1024 * 1024
}
#[cfg(test)]
pub(crate) fn used_capacity_for_test() -> usize {
    CAPACITY.used()
}
#[cfg(test)]
fn capacity(
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<crate::byte_budget::Lease> {
    crate::bounded_worker_process::capacity(&CAPACITY, CHILD_LIMIT, deadline, stopped)
}
pub fn spawn(
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
pub fn spawn_svg(
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
pub fn spawn_session(
    codec: crate::surface_image_worker_wire::Codec,
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<Child> {
    let suffix = match (codec, cfg!(test)) {
        (crate::surface_image_worker_wire::Codec::Svg, true) => {
            " --exact surface_svg_worker::tests::svg_session_worker_entry --ignored --nocapture"
        }
        (crate::surface_image_worker_wire::Codec::Svg, false) => " --native-svg-session-worker",
        (crate::surface_image_worker_wire::Codec::WebP, true) => {
            " --exact surface_webp_worker::tests::webp_session_worker_entry --ignored --nocapture"
        }
        (crate::surface_image_worker_wire::Codec::WebP, false) => " --native-webp-session-worker",
    };
    start(endpoint, deadline, stopped, suffix)
}
pub fn start(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
    suffix: &str,
) -> io::Result<Child> {
    crate::bounded_worker_process::start_reclaiming(
        endpoint,
        deadline,
        stopped,
        crate::bounded_worker_process::Policy {
            suffix,
            maximum: CHILD_LIMIT,
            pool: &CAPACITY,
            environment: crate::bounded_worker_process::Environment::Empty,
        },
        &crate::surface_image_worker_session::reclaim_idle,
    )
}
