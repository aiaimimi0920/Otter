//! Capture latest frame -> original GPU readback -> fragmented media; bounded same-Consumer/sequence recovery without reopening capture or replaying lost AUs.
use super::*;
use crate::{
    live_relay_frame::{Codec, ColorSpace, Metadata},
    live_relay_media_transport::SourceStep,
};
use std::time::{Duration, Instant};

pub(super) fn run(
    client: &crate::device_session_client::Client,
    lease: &Lease,
    publication: &Publication,
    mut consumer: Consumer<Texture>,
    budget: &Arc<Budget>,
    signal: &Arc<Signal>,
    mut video: video::Video,
) -> Result<(), Exit> {
    let _buffers = budget
        .reserve(crate::live_relay_media_transport::BUFFER_BUDGET)
        .ok_or(Exit::Budget)?;
    let mut socket = None;
    let mut recovery = recovery::Recovery::default();
    let current = std::thread::current();
    consumer
        .set_waker(move || {
            current.unpark();
            true
        })
        .map_err(|_| Exit::SourceClosed)?;
    let mut next_frame = Instant::now();
    let mut next_control = Instant::now();
    let mut frame_id = 0u64;
    let mut last_sequence = None;
    // Retain only one Arc to an existing immutable GPU frame for idle-image IDR requests; no extra capture or pixel backlog.
    let mut latest = None;
    // Performance audit optimization (Tier 0.1):
    // Reuse a single D3D11 staging texture across frames via `readback::Scratch`, avoiding 8.3–33.2 MB system-RAM
    // page lock/unlock churn on every remote stream frame; clear on disconnect/retry so idle sockets hold zero VRAM/RAM.
    let mut readback_scratch = crate::live_texture::readback::Scratch::default();
    loop {
        recovery::check(client, lease, signal)?;
        if socket.is_none() {
            match recovery::connect(client, lease, publication, signal, &mut video) {
                Ok(connected) => {
                    socket = Some(connected);
                    if recovery.active() {
                        video.reconnect();
                    }
                    next_frame = Instant::now();
                    next_control = Instant::now();
                }
                Err(exit) => {
                    readback_scratch.clear();
                    signal.retire_media();
                    if signal.stop.load(Ordering::Acquire) {
                        return Err(Exit::Stopped);
                    }
                    let exit = if exit
                        == Exit::Media(crate::live_relay_media_transport::Error::Stopped)
                        && signal.restart_media.load(Ordering::Acquire)
                    {
                        // Incomplete handshake does not prove the service saw Source exit; retain restart requests for consumption by an upgraded connection.
                        recovery::media_restart()
                    } else {
                        exit
                    };
                    recovery.retry(exit, client, lease, signal)?;
                    continue;
                }
            }
        }
        let clock = Instant::now();
        let mut idle = false;
        let result = (|| -> Result<(), Exit> {
            if signal.restart_media.swap(false, Ordering::AcqRel) {
                return Err(recovery::media_restart());
            }
            let socket = socket.as_mut().unwrap();
            if clock >= next_control {
                for _ in 0..4 {
                    match socket.poll_source().map_err(Exit::Media)? {
                        SourceStep::Idle => break,
                        SourceStep::Policy(policy) => video.policy(policy).map_err(|_| {
                            Exit::Media(crate::live_relay_media_transport::Error::Profile)
                        })?,
                        SourceStep::Closed => return Err(Exit::RemoteClosed),
                        SourceStep::Revoked => return Err(Exit::Revoked),
                    }
                }
                next_control = Instant::now() + Duration::from_millis(100);
            }
            if clock >= next_frame {
                let delivery = consumer.try_next().map_err(|_| Exit::SourceClosed)?;
                let refresh = video.refresh();
                let changed = delivery.is_some();
                if let Some(delivery) = delivery {
                    latest = Some((delivery.frame, now()?));
                }
                if let Some((resource, capture)) = latest.as_ref().filter(|_| changed || refresh) {
                    let sequence = resource.sequence;
                    let frame = readback_scratch
                        .read(resource.clone(), &signal.stop)
                        .map_err(|_| Exit::Readback)?;
                    video.resize([frame.width, frame.height], budget);
                    frame_id = frame_id.checked_add(1).ok_or(Exit::Geometry)?;
                    let dropped = last_sequence
                        .map_or(0, |old: u64| sequence.saturating_sub(old).saturating_sub(1));
                    let encoded = video.encode(&frame.pixels, frame_id, &signal.stop);
                    if signal.stop.load(Ordering::Acquire) {
                        return Err(Exit::Stopped);
                    }
                    let metadata = Metadata {
                        epoch: 1,
                        frame_id,
                        capture_timestamp_ms: *capture,
                        encode_timestamp_ms: now()?,
                        width: frame.width,
                        height: frame.height,
                        dropped_frames: u32::try_from(dropped).unwrap_or(u32::MAX),
                        keyframe: encoded.as_ref().is_none_or(|e| e.keyframe),
                        color_space: ColorSpace::Srgb,
                        codec: if encoded.is_some() {
                            Codec::H264
                        } else {
                            Codec::RawBgra
                        },
                    };
                    socket
                        .send_source(
                            metadata,
                            encoded.as_ref().map_or(&frame.pixels[..], |e| &e.bytes),
                            || {
                                !signal.stop.load(Ordering::Acquire)
                                    && now().is_ok_and(|now| {
                                        lease.validate(client.origin(), now).is_ok()
                                    })
                            },
                        )
                        .map_err(Exit::Media)?;
                    last_sequence = Some(sequence);
                    recovery.confirm(client, lease, publication, signal, frame_id)?;
                    if !signal.ready.swap(true, Ordering::AcqRel) {
                        let _ = unsafe { SetEvent(HANDLE(signal.event.as_raw_handle())) };
                    }
                    next_frame = Instant::now()
                        + Duration::from_micros(if publication.h264() { 33_334 } else { 16_667 });
                } else {
                    idle = true;
                }
            }
            Ok(())
        })();
        if let Err(exit) = result {
            // Consume rotation only while actively retiring an upgraded connection. Ordinary I/O failure cannot clear a slightly later request.
            // Retire the old socket/interrupt first; unknown-completion frame numbers are consumed, and the next connection sends only fresh raw/IDR.
            socket.take();
            readback_scratch.clear();
            signal.retire_media();
            recovery.retry(exit, client, lease, signal)?;
            continue;
        }
        // Without frames, wait only for original Consumer notices; the 100ms bound services sockets/authorization without extra screenshots.
        let wake = if idle {
            next_control
        } else {
            next_frame.min(next_control)
        };
        std::thread::park_timeout(wake.saturating_duration_since(Instant::now()));
    }
}
