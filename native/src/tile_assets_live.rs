//! Images and at most four Wall streams use the original Assets thread; Art has independent bounded HTTP, publishing only complete canvases whose identity is still current.
use super::*;
use crate::{
    image_decode::Image,
    tile_live_packet::WallMediaFormat,
    tile_live_queue::Queue,
    tile_live_socket::{Interrupts, Receiver},
    tile_shared::Candidate,
};
use std::time::{Duration, Instant};
const MAX_LIVE: usize = 4;

struct Connections<'a> {
    streams: BTreeMap<String, Receiver>,
    interrupts: &'a Interrupts,
}
impl Drop for Connections<'_> {
    fn drop(&mut self) {
        self.streams.clear();
        self.interrupts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Active,
    Held,
    Ended,
}
pub(crate) fn phase(s: &crate::tile_shared::State, job: &Job) -> Phase {
    if s.stopped
        || s.expired
        || s.generation != job.generation
        || s.desired.as_ref() != Some(&job.key)
    {
        Phase::Ended
    } else if s.frozen || s.identification.blocked() {
        Phase::Held
    } else if s.black {
        Phase::Ended
    } else {
        Phase::Active
    }
}
pub(crate) fn current(shared: &Shared, job: &Job) -> bool {
    phase(&shared.lock(), job) == Phase::Active
}
// Interpret cancellation and completion after the same generation fence; freeze does not upgrade HTTP/Live cancellation into view retirement.
fn after_poll(
    shared: &Shared,
    job: &Job,
    result: Result<(), &'static str>,
) -> Result<Phase, &'static str> {
    let phase = phase(&shared.lock(), job);
    if phase == Phase::Active {
        result?;
    }
    Ok(phase)
}
pub(super) fn run(
    client: &Client,
    lease: &Arc<Lease>,
    art_lease: &Arc<Lease>,
    shared: &Arc<Shared>,
    job: &Job,
    cache: &mut BTreeMap<String, Image>,
    interrupts: &Interrupts,
) -> Result<(), &'static str> {
    let tile = job
        .layout
        .tiles
        .iter()
        .find(|t| t.endpoint_id == job.key.endpoint)
        .ok_or("tile-output-unassigned")?;
    let mut kinds = BTreeMap::new();
    for p in job
        .layout
        .placements
        .iter()
        .filter(|p| tile.rect.intersects(&p.rect))
    {
        if kinds
            .insert(p.source.id.clone(), p.source.kind.clone())
            .is_some_and(|k| k != p.source.kind)
        {
            return Err("tile-source-identity-conflict");
        }
    }
    let ids: BTreeSet<_> = kinds
        .iter()
        .filter(|(_, k)| *k == "image")
        .map(|(id, _)| id.clone())
        .collect();
    let live: Vec<_> = kinds
        .iter()
        .filter(|(_, k)| *k == "live")
        .map(|(id, _)| id.clone())
        .collect();
    let art: Vec<_> = kinds
        .iter()
        .filter(|(_, k)| *k == "surface")
        .map(|(id, _)| id.clone())
        .collect();
    let art_placements = job
        .layout
        .placements
        .iter()
        .filter(|p| p.source.kind == "surface" && tile.rect.intersects(&p.rect))
        .count();
    if ids.len() > 16 || live.len() > MAX_LIVE || art.len() > 4 || art_placements > 16 {
        return Err("tile-source-count-limit");
    }
    cache.retain(|k, _| ids.contains(k));
    for id in ids {
        if !current(shared, job) {
            return Ok(());
        }
        if !cache.contains_key(&id) {
            let image = lease
                .control(client)
                .map_err(|_| "tile-authorization-invalid")?
                .wall_image(&job.key.endpoint, &job.key.lease, job.key.revision, &id)
                .map_err(|_| "tile-image-read-failed")?;
            if pixels(cache.values()) + area(&image) > crate::tile_wire::MAX_PIXELS {
                return Err("tile-image-pixel-budget");
            }
            cache.insert(id, image);
        }
    }
    let mut connections = Connections {
        streams: BTreeMap::new(),
        interrupts,
    };
    let image_pixels = pixels(cache.values());
    let mut worker = crate::tile_surface_worker::Worker::start(
        client,
        art_lease,
        shared,
        job,
        art.clone(),
        image_pixels,
    )?;
    let empty_arts = BTreeMap::new();
    loop {
        if phase(&shared.lock(), job) == Phase::Held {
            // Freeze preserves only the actually presented frame and view, without Art/image polling or Tile media connections; it does not cancel source-accepted actions.
            connections.streams.clear();
            connections
                .interrupts
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            if !shared.wait(Duration::from_millis(50)) {
                return Ok(());
            }
            continue;
        }
        if !current(shared, job)
            || lease
                .validate(client.origin(), crate::tile_runtime::now_ms())
                .is_err()
        {
            return Ok(());
        }
        let result = (|| {
            for id in &live {
                if !connections.streams.contains_key(id) {
                    let receiver = Receiver::connect(
                        client,
                        lease,
                        &job.key,
                        id,
                        WallMediaFormat::RawBgra,
                        interrupts,
                    )?;
                    connections.streams.insert(id.clone(), receiver);
                }
            }
            for stream in connections.streams.values_mut() {
                stream.poll(|| !current(shared, job) || lease.is_revoked())?;
            }
            worker.check()
        })();
        match after_poll(shared, job, result)? {
            Phase::Held => continue,
            Phase::Ended => return Ok(()),
            Phase::Active => {}
        }
        let now = Instant::now();
        let (estimate, prepared) = {
            let state = shared.lock();
            (state.clock.estimate(now), state.art.clone())
        };
        let arts = prepared.as_ref().map_or(&empty_arts, |a| &a.frames);
        let mut complete = arts.len() == art.len();
        let mut total_pixels = image_pixels + prepared.as_ref().map_or(0, |a| a.pixels);
        let mut until = None;
        // Polling advances and expires every queue even if the complete frame is unchanged.
        let mut selected = [None; MAX_LIVE];
        for ((id, stream), slot) in connections.streams.iter_mut().zip(&mut selected) {
            if let Some(frame) = stream.queue.advance(estimate) {
                let deadline = Queue::deadline(frame, estimate.unwrap(), now);
                until = Some(until.map_or(deadline, |v: Instant| v.min(deadline)));
                total_pixels += area(&frame.image);
                *slot = Some((id, &frame.image));
            } else {
                complete = false;
            }
        }
        if total_pixels > crate::tile_wire::MAX_PIXELS {
            return Err("tile-content-pixel-budget");
        }
        if complete {
            shared.publish_candidate(
                job.generation,
                Candidate {
                    key: &job.key,
                    layout: &job.layout,
                    size: job.size,
                    images: cache.iter().chain(selected.iter().flatten().copied()),
                    arts,
                    until,
                },
                prepared.as_ref(),
            );
        } else {
            shared.unavailable(job.generation, &job.key, "tile-content-waiting");
        }
        if (connections.streams.is_empty() && art.is_empty())
            || !shared.wait(Duration::from_millis(16))
        {
            return Ok(());
        }
    }
}
fn area(i: &Image) -> u64 {
    u64::from(i.width) * u64::from(i.height)
}
fn pixels<'a>(images: impl Iterator<Item = &'a Image>) -> u64 {
    images.map(area).sum()
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "tile_assets_live_tests.rs"]
mod tests;
