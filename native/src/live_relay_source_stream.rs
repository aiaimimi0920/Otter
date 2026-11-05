//! 捕获最新帧→原GPU读回→分片媒体；同Consumer/序号有限恢复，不重开捕获或重放丢失AU。
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
    // 仅保留一份已有不可变GPU帧的Arc，供静止画面收到IDR请求时重发；不另截图或积压像素。
    let mut latest = None;
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
                    signal
                        .socket
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .take();
                    recovery.retry(exit, client, lease, signal)?;
                    continue;
                }
            }
        }
        let clock = Instant::now();
        let mut idle = false;
        let result = (|| -> Result<(), Exit> {
            if signal.restart_media.swap(false, Ordering::AcqRel) {
                return Err(Exit::Media(
                    crate::live_relay_media_transport::Error::Media(
                        crate::live_relay_media_socket::Error::Liveness,
                    ),
                ));
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
                    if [resource.resource.width, resource.resource.height] != publication.size() {
                        return Err(Exit::Geometry);
                    }
                    let frame = crate::live_texture::readback::read(resource.clone(), &signal.stop)
                        .map_err(|_| Exit::Readback)?;
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
            if matches!(exit, Exit::Media(e) if e.recoverable()) {
                signal.restart_media.store(false, Ordering::Release);
            }
            // 旧socket和其interrupt先退休；未知完成的帧号已消费，下一连接只发新raw/IDR。
            socket.take();
            signal
                .socket
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take();
            recovery.retry(exit, client, lease, signal)?;
            continue;
        }
        // 无帧只等原Consumer通知；100ms上限服务socket/授权，不做额外截图。
        let wake = if idle {
            next_control
        } else {
            next_frame.min(next_control)
        };
        std::thread::park_timeout(wake.saturating_duration_since(Instant::now()));
    }
}
