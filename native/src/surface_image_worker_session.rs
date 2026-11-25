//! Short-lived warm session pool for same-origin Surface batch image decoding: exit after 4s idle, rotate at 32 images, and isolate with a 128MiB Job.
use crate::{
    bounded_worker_process::session as boundary,
    image_decode::{pixel_buffer, Image},
    live_recovery_pipe::Pipe,
    surface_image_worker_process as process, surface_image_worker_wire as wire,
};
use std::{
    io::{self, Read, Write},
    os::windows::io::AsRawHandle,
    sync::{Condvar, LazyLock, Mutex},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_TIMEOUT},
    System::Threading::WaitForSingleObject,
};

const IDLE_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_REQUESTS: usize = 32;
const MAX_PARKED: usize = 2;

struct Parked {
    scope: String,
    codec: wire::Codec,
    pipe: Pipe,
    child: process::Child,
    requests: usize,
    expires_at: Instant,
}
impl Parked {
    fn alive(&self) -> bool {
        unsafe {
            WaitForSingleObject(HANDLE(self.child.handle().as_raw_handle()), 0) == WAIT_TIMEOUT
        }
    }
}

#[derive(Default)]
struct PoolState {
    entries: Vec<Parked>,
    reaper_running: bool,
}

static POOL: LazyLock<(Mutex<PoolState>, Condvar)> =
    LazyLock::new(|| (Mutex::new(PoolState::default()), Condvar::new()));

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid image worker session response",
    )
}

pub(crate) fn decode_scoped(
    scope: &str,
    bytes: &[u8],
    codec: wire::Codec,
    expected: Option<(u32, u32)>,
    stopped: &impl Fn() -> bool,
) -> io::Result<(Image, [f32; 2])> {
    decode_scoped_with_idle(scope, bytes, codec, expected, IDLE_TIMEOUT, stopped)
}

fn decode_scoped_with_idle(
    scope: &str,
    bytes: &[u8],
    codec: wire::Codec,
    expected: Option<(u32, u32)>,
    idle: Duration,
    stopped: &impl Fn() -> bool,
) -> io::Result<(Image, [f32; 2])> {
    if scope.is_empty()
        || scope.len() > 512
        || scope.chars().any(char::is_control)
        || bytes.is_empty()
        || bytes.len() > wire::MAX_INPUT
        || stopped()
    {
        return Err(invalid());
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut session = take_matching(scope, codec);
    if session.is_none() {
        let (pipe, endpoint) = Pipe::pair()?;
        let child = process::spawn_session(codec, endpoint.into_handle(), deadline, stopped)?;
        session = Some(Parked {
            scope: scope.to_owned(),
            codec,
            pipe,
            child,
            requests: 0,
            expires_at: Instant::now() + idle,
        });
    }
    let mut active = session.unwrap();
    let result = (|| -> io::Result<(u32, u32, [f32; 2], Vec<u8>, crate::byte_budget::Lease)> {
        let mut channel = active
            .pipe
            .cancellable_io(deadline, active.child.handle(), stopped);
        channel.write_all(codec.request())?;
        channel.write_all(&(bytes.len() as u32).to_le_bytes())?;
        channel.write_all(bytes)?;
        channel.flush()?;
        let mut header = [0; 16];
        channel.read_exact(&mut header)?;
        let (width, height, count) = wire::response_for(codec, &header)?;
        if expected.is_some_and(|size| size != (width, height)) {
            return Err(invalid());
        }
        let logical_size = if codec == wire::Codec::Svg {
            let mut metadata = [0; 8];
            channel.read_exact(&mut metadata)?;
            wire::logical_size(
                [
                    f32::from_le_bytes(metadata[..4].try_into().unwrap()),
                    f32::from_le_bytes(metadata[4..].try_into().unwrap()),
                ],
                [width, height],
            )?
        } else {
            [width as f32, height as f32]
        };
        let (mut pixels, lease) = pixel_buffer(count).map_err(io::Error::other)?;
        channel.read_exact(&mut pixels)?;
        Ok((width, height, logical_size, pixels, lease))
    })();
    let (width, height, logical_size, pixels, lease) = match result {
        Ok(ok) => ok,
        Err(error) => {
            drop(active);
            return Err(error);
        }
    };
    if stopped() {
        drop(active);
        return Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Image cancelled",
        ));
    }
    active.requests += 1;
    if active.requests < MAX_REQUESTS {
        boundary::idle(&active.pipe, &active.child)?;
        active.expires_at = Instant::now() + idle;
        park(active);
    } else {
        boundary::finish(&mut active.pipe, &active.child, deadline, stopped)?;
        drop(active);
    }
    if stopped() {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Image cancelled",
        ));
    }
    Ok((
        Image {
            width,
            height,
            pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
        },
        logical_size,
    ))
}

fn take_matching(scope: &str, codec: wire::Codec) -> Option<Parked> {
    let now = Instant::now();
    let (mut expired, mut found) = (Vec::new(), None);
    if let Ok(mut state) = POOL.0.lock() {
        let mut i = 0;
        while i < state.entries.len() {
            if state.entries[i].expires_at <= now || !state.entries[i].alive() {
                expired.push(state.entries.swap_remove(i));
            } else if found.is_none()
                && state.entries[i].scope == scope
                && state.entries[i].codec == codec
            {
                found = Some(state.entries.swap_remove(i));
            } else {
                i += 1;
            }
        }
    }
    drop(expired);
    found.filter(|entry| boundary::idle(&entry.pipe, &entry.child).is_ok())
}

/// Active sessions are absent from entries; each admission round reclaims only returned idle owners when the pool is full.
pub(crate) fn reclaim_idle() {
    let victim = POOL.0.lock().ok().and_then(|mut state| {
        if process::at_capacity() {
            state
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.expires_at)
                .map(|(idx, _)| idx)
                .map(|idx| state.entries.swap_remove(idx))
        } else {
            None
        }
    });
    drop(victim);
}

fn park(entry: Parked) {
    park_with(entry, || {
        std::thread::Builder::new()
            .name("otter-img-idle-reap".into())
            .spawn(reap_loop)
            .map(|_| ())
    });
}

fn park_with(entry: Parked, spawn: impl FnOnce() -> io::Result<()>) {
    let mut spawn_reaper = false;
    let mut evicted = Vec::new();
    if let Ok(mut state) = POOL.0.lock() {
        while state.entries.len() >= MAX_PARKED {
            if let Some((idx, _)) = state
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.expires_at)
            {
                evicted.push(state.entries.swap_remove(idx));
            }
        }
        state.entries.push(entry);
        if !state.reaper_running {
            state.reaper_running = true;
            spawn_reaper = true;
        }
        POOL.1.notify_all();
    }
    drop(evicted);
    if spawn_reaper && spawn().is_err() {
        // No other spawner exists while reserved is true; rollback and orphan removal share one lock, with destruction outside the pool lock.
        let orphaned = {
            let mut state = POOL.0.lock().unwrap_or_else(|e| e.into_inner());
            state.reaper_running = false;
            std::mem::take(&mut state.entries)
        };
        drop(orphaned);
    }
}

fn reap_loop() {
    loop {
        let mut expired = Vec::new();
        let wait = {
            let mut guard = POOL.0.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            let mut i = 0;
            while i < guard.entries.len() {
                if guard.entries[i].expires_at <= now || !guard.entries[i].alive() {
                    expired.push(guard.entries.swap_remove(i));
                } else {
                    i += 1;
                }
            }
            if expired.is_empty() {
                if let Some(next) = guard.entries.iter().map(|e| e.expires_at).min() {
                    let delay = next.saturating_duration_since(now);
                    let _ = POOL.1.wait_timeout(guard, delay);
                    None
                } else {
                    guard.reaper_running = false;
                    return;
                }
            } else {
                Some(())
            }
        };
        drop(expired);
        let _ = wait;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::io::AsHandle;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use windows::Win32::System::Threading::GetProcessId;
    const SVG: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'><rect width='4' height='4' fill='#ff0000'/></svg>";

    fn parked_pid(scope: &str, codec: wire::Codec) -> Option<u32> {
        let guard = POOL.0.lock().unwrap();
        guard
            .entries
            .iter()
            .find(|e| e.scope == scope && e.codec == codec)
            .map(|e| unsafe { GetProcessId(HANDLE(e.child.handle().as_raw_handle())) })
    }

    #[test]
    fn webp_session_reuses_exact_pixels_and_rotates_after_32_requests() {
        let bytes = Image::webp_for_test();
        let scope = "https://webp-session.example";
        let mut first_pid = None;
        let mut first_handle = None;
        for index in 0..=MAX_REQUESTS {
            let (image, logical) =
                Image::from_surface_sized_scoped(Some(scope), &bytes, "image/webp", None, || false)
                    .unwrap();
            assert_eq!(logical, [2., 1.]);
            assert_eq!(&image.pixels[..], &[0, 0, 255, 255, 0, 128, 0, 128]);
            let pid = parked_pid(scope, wire::Codec::WebP);
            if index == 0 {
                first_pid = pid;
                assert!(first_pid.is_some_and(|pid| pid != 0));
                first_handle = Some(
                    POOL.0
                        .lock()
                        .unwrap()
                        .entries
                        .iter()
                        .find(|e| e.scope == scope)
                        .unwrap()
                        .child
                        .handle()
                        .as_handle()
                        .try_clone_to_owned()
                        .unwrap(),
                );
            } else if index < MAX_REQUESTS - 1 {
                assert_eq!(pid, first_pid);
            } else if index == MAX_REQUESTS - 1 {
                assert_eq!(pid, None);
                assert_eq!(process::used_capacity_for_test(), 0);
            } else {
                // The original process handle remains alive, so Windows cannot reuse its PID and mask rotation.
                assert!(first_handle.is_some());
                assert!(pid.is_some_and(|pid| pid != 0));
                assert_ne!(pid, first_pid);
            }
        }
        drop(take_matching(scope, wire::Codec::WebP));
    }

    #[test]
    fn oversized_pool_key_falls_back_without_rejecting_inline_images() {
        let scope = format!("https://{}.example", "a".repeat(600));
        for (bytes, mime, size) in [
            (SVG.to_vec(), "image/svg+xml", [4., 4.]),
            (Image::webp_for_test(), "image/webp", [2., 1.]),
        ] {
            let (image, logical) =
                Image::from_surface_sized_scoped(Some(&scope), &bytes, mime, None, || false)
                    .unwrap();
            assert_eq!(logical, size);
            assert!(!image.pixels.is_empty());
            assert_eq!(process::used_capacity_for_test(), 0);
        }
    }

    #[test]
    fn same_origin_svg_batch_reuses_worker_isolated_by_origin_and_auto_expires() {
        let svg_a = b"<svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'><rect width='4' height='4' fill='#ff0000'/></svg>";
        let svg_b = b"<svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'><rect width='4' height='4' fill='#00ff00'/></svg>";
        let idle = Duration::from_millis(180);
        let (first, _) = decode_scoped_with_idle(
            "https://origin-a.example",
            svg_a,
            wire::Codec::Svg,
            None,
            idle,
            &|| false,
        )
        .unwrap();
        assert_eq!(&first.pixels[..4], &[0, 0, 255, 255]);
        let pid_a1 = parked_pid("https://origin-a.example", wire::Codec::Svg).unwrap();
        let (second, _) = decode_scoped_with_idle(
            "https://origin-a.example",
            svg_b,
            wire::Codec::Svg,
            None,
            idle,
            &|| false,
        )
        .unwrap();
        assert_eq!(&second.pixels[..4], &[0, 255, 0, 255]);
        let pid_a2 = parked_pid("https://origin-a.example", wire::Codec::Svg).unwrap();
        assert_eq!(pid_a1, pid_a2);

        let (third, _) = decode_scoped_with_idle(
            "https://origin-b.example",
            svg_a,
            wire::Codec::Svg,
            None,
            idle,
            &|| false,
        )
        .unwrap();
        assert_eq!(&third.pixels[..4], &[0, 0, 255, 255]);
        let pid_b = parked_pid("https://origin-b.example", wire::Codec::Svg).unwrap();
        assert_ne!(pid_a1, pid_b);

        std::thread::sleep(Duration::from_millis(260));
        assert_eq!(process::used_capacity_for_test(), 0);
    }

    #[test]
    fn svg_hot_and_cold_waiters_reclaim_newly_parked_active_worker() {
        for cold in [false, true] {
            for scope in ["active-a", "active-b"] {
                decode_scoped_with_idle(
                    scope,
                    SVG,
                    wire::Codec::Svg,
                    None,
                    Duration::from_secs(25),
                    &|| false,
                )
                .unwrap();
            }
            let first = take_matching("active-a", wire::Codec::Svg).unwrap();
            let second = take_matching("active-b", wire::Codec::Svg).unwrap();
            let (waiting, observed) = std::sync::mpsc::channel();
            std::thread::scope(|threads| {
                let waiter = threads.spawn(|| {
                    let checks = AtomicUsize::new(0);
                    let stopped = || {
                        if checks.fetch_add(1, Ordering::Relaxed) == 4 {
                            waiting.send(()).unwrap();
                        }
                        false
                    };
                    if cold {
                        Image::from_surface_sized(SVG, "image/svg+xml", None, stopped)
                            .map_err(io::Error::other)
                    } else {
                        decode_scoped("waiting", SVG, wire::Codec::Svg, None, &stopped)
                    }
                });
                observed.recv_timeout(Duration::from_secs(3)).unwrap();
                assert!(process::at_capacity());
                assert!(first.alive() && second.alive());
                let parked_at = Instant::now();
                park(first);
                let (image, _) = waiter.join().unwrap().unwrap();
                assert_eq!(&image.pixels[..4], &[0, 0, 255, 255]);
                assert!(parked_at.elapsed() < Duration::from_secs(5));
                assert!(second.alive(), "active peer must remain untouched");
            });
            drop(second);
            drop(take_matching("waiting", wire::Codec::Svg));
            assert_eq!(process::used_capacity_for_test(), 0);
        }
    }

    #[test]
    fn svg_reaper_spawn_failure_releases_parked_capacity_and_resets_reservation() {
        let (pipe, endpoint) = Pipe::pair().unwrap();
        let child = process::spawn_session(
            wire::Codec::Svg,
            endpoint.into_handle(),
            Instant::now() + Duration::from_secs(5),
            &|| false,
        )
        .unwrap();
        park_with(
            Parked {
                scope: "failed-reaper".into(),
                codec: wire::Codec::Svg,
                pipe,
                child,
                requests: 0,
                expires_at: Instant::now() + IDLE_TIMEOUT,
            },
            || Err(io::Error::other("controlled reaper spawn failure")),
        );
        let state = POOL.0.lock().unwrap();
        assert!(!state.reaper_running);
        assert!(state.entries.is_empty());
        assert_eq!(process::used_capacity_for_test(), 0);
    }
}
