//! 图片与最多四个Wall流沿原Assets线程；Art独立有界HTTP，只有完整且身份仍当前的画布才发布。
use super::*;
use crate::{
    image_decode::Image,
    tile_live_packet::WallMediaFormat,
    tile_live_queue::Queue,
    tile_live_socket::{Interrupts, Receiver},
    tile_shared::Frame,
};
use std::time::{Duration, Instant};
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
// 取消与完成都在同一代际围栏之后解释；冻结不把HTTP/Live取消升级为view退休。
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
        .ok_or("输出未分配")?;
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
            return Err("可见来源身份冲突");
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
    if ids.len() > 16 || live.len() > 4 || art.len() > 4 || art_placements > 16 {
        return Err("可见来源数量超限");
    }
    cache.retain(|k, _| ids.contains(k));
    for id in ids {
        if !current(shared, job) {
            return Ok(());
        }
        if !cache.contains_key(&id) {
            let image = lease
                .control(client)
                .map_err(|_| "设备授权失效")?
                .wall_image(&job.key.endpoint, &job.key.lease, job.key.revision, &id)
                .map_err(|_| "图片读取或校验失败")?;
            if pixels(cache.values()) + area(&image) > crate::tile_wire::MAX_PIXELS {
                return Err("可见图片像素预算不足");
            }
            cache.insert(id, image);
        }
    }
    let mut connections = Connections {
        streams: BTreeMap::new(),
        interrupts,
    };
    let mut worker = crate::tile_surface_worker::Worker::start(
        client,
        art_lease,
        shared,
        job,
        art.clone(),
        pixels(cache.values()),
    )?;
    loop {
        if phase(&shared.lock(), job) == Phase::Held {
            // Freeze只保实际呈现帧和view，不轮询Art/图片、不保Tile媒体连接；不取消源已接受动作。
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
        let estimate = shared.lock().clock.estimate(now);
        let mut images = cache.clone();
        let mut until = None;
        let prepared = shared.lock().art.clone();
        let arts = prepared
            .as_ref()
            .map_or_else(BTreeMap::new, |a| a.frames.clone());
        let mut complete = arts.len() == art.len();
        for (id, stream) in &mut connections.streams {
            if let Some(frame) = stream.queue.advance(estimate) {
                let deadline = Queue::deadline(frame, estimate.unwrap(), now);
                until = Some(until.map_or(deadline, |v: Instant| v.min(deadline)));
                images.insert(id.clone(), frame.image.clone());
            } else {
                complete = false;
            }
        }
        if pixels(images.values()) + prepared.as_ref().map_or(0, |a| a.pixels)
            > crate::tile_wire::MAX_PIXELS
        {
            return Err("可见内容像素预算不足");
        }
        if complete {
            shared.publish_prepared(
                job.generation,
                Frame {
                    key: job.key.clone(),
                    layout: job.layout.clone(),
                    size: job.size,
                    images,
                    arts,
                    live_until: None,
                },
                until,
                prepared.as_ref(),
            );
        } else {
            shared.unavailable(job.generation, &job.key, "正在等待完整内容");
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
#[cfg(test)]
#[path = "tile_assets_live_tests.rs"]
mod tests;
