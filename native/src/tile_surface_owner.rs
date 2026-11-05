//! 独占Art串行请求线程内最多四个ephemeral view；纯数据交给UI，HTTP/资源owner不跨线程。
use crate::{
    device_authorization_lease::Lease,
    device_session_client::Client,
    image_decode::Image,
    tile_assets::Job,
    tile_shared::Shared,
    tile_surface_wire::{Binding, Snapshot, State},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};
pub(crate) struct Art {
    pub view: crate::tile_surface_wire::View,
    pub generation: u64,
    pub snapshot: Arc<Snapshot>,
    pub size: [u32; 2],
    pub images: BTreeMap<String, Image>,
}
struct Entry {
    state: Arc<State>,
    ready: Option<Arc<Art>>,
    next: Instant,
}
enum Task {
    State(String),
    Image(String, String),
}
fn task(
    entries: &BTreeMap<String, Entry>,
    images: &BTreeMap<String, Image>,
    ids: &[String],
) -> Option<Task> {
    // 一步只下载一张；不排额外采样/资源任务，原Live在独立Assets线程继续消费。
    for (id, entry) in entries {
        if let Some(resource) = entry
            .state
            .snapshot
            .dimensions
            .keys()
            .find(|r| !images.contains_key(*r))
        {
            return Some(Task::Image(id.clone(), resource.clone()));
        }
    }
    let id = ids
        .iter()
        .min_by_key(|id| entries.get(*id).map(|e| e.next))?;
    entries
        .get(id)
        .is_none_or(|e| Instant::now() >= e.next)
        .then(|| Task::State(id.clone()))
}
// 持Shared锁接纳场景并清旧候选；冻结/换代期间迟到的state不能替换保留场景。
fn admit(
    entries: &mut BTreeMap<String, Entry>,
    id: String,
    state: State,
    shared: &Shared,
    job: &Job,
) -> Option<State> {
    let mut s = shared.lock();
    if crate::tile_assets::live::phase(&s, job) != crate::tile_assets::live::Phase::Active {
        return Some(state);
    }
    let ready = entries
        .get(&id)
        .and_then(|e| e.ready.as_ref())
        .filter(|r| {
            Arc::ptr_eq(&r.snapshot, &state.snapshot)
                && r.view == state.view
                && r.generation == state.generation
        })
        .cloned();
    if entries.get(&id).is_some_and(|e| {
        !Arc::ptr_eq(&e.state.snapshot, &state.snapshot) || e.state.generation != state.generation
    }) {
        s.art = None;
        s.unavailable(job.generation, &job.key, "正在准备Art内容");
    }
    entries.insert(
        id,
        Entry {
            state: Arc::new(state),
            ready,
            next: Instant::now() + Duration::from_millis(350),
        },
    );
    None
}
fn dimensions_match(dims: &[Option<u32>; 2], image: &Image) -> bool {
    dims.iter()
        .zip([image.width, image.height])
        .all(|(d, actual)| d.is_none_or(|d| d == actual))
}
pub(crate) struct Views<'a> {
    client: &'a Client,
    lease: &'a Lease,
    entries: BTreeMap<String, Entry>,
    // 内容寻址只在本job/view授权域内共享；换代整体释放，不继承旧授权。
    images: BTreeMap<String, Image>,
}
impl<'a> Views<'a> {
    pub fn new(client: &'a Client, lease: &'a Lease) -> Self {
        Self {
            client,
            lease,
            entries: BTreeMap::new(),
            images: BTreeMap::new(),
        }
    }
    pub fn frames(&self) -> BTreeMap<String, Arc<Art>> {
        self.entries
            .iter()
            .filter_map(|(id, e)| e.ready.clone().map(|r| (id.clone(), r)))
            .collect()
    }
    pub fn pixels(&self) -> u64 {
        self.images
            .values()
            .map(|i| u64::from(i.width) * u64::from(i.height))
            .sum()
    }
    pub fn poll(
        &mut self,
        ids: &[String],
        shared: &Shared,
        job: &Job,
        other_pixels: u64,
    ) -> Result<(), &'static str> {
        if ids.len() > 4 {
            return Err("可见Art数量超限");
        }
        if !super::tile_assets::live::current(shared, job) {
            return Ok(());
        }
        let Some(task) = task(&self.entries, &self.images, ids) else {
            return Ok(());
        };
        let binding = Binding::from(&job.key);
        let control = self
            .lease
            .control(self.client)
            .map_err(|_| "设备授权失效")?;
        if let Task::Image(id, resource) = task {
            let entry = &self.entries[&id];
            let image = control
                .wall_surface_image(&entry.state.view, &resource)
                .map_err(|_| "Art图片读取失败")?;
            if !super::tile_assets::live::current(shared, job) {
                return Ok(());
            }
            if !dimensions_match(&entry.state.snapshot.dimensions[&resource], &image) {
                return Err("Art图片尺寸不匹配");
            }
            let pixels = u64::from(image.width) * u64::from(image.height);
            if other_pixels + self.pixels() + pixels > crate::tile_wire::MAX_PIXELS {
                return Err("可见内容像素预算不足");
            }
            self.images.insert(resource, image);
            return self.prepare();
        }
        let Task::State(id) = task else {
            unreachable!()
        };
        let old = self.entries.get(&id);
        let bytes = if let Some(entry) = old {
            control.wall_surface_state(&entry.state.view, entry.state.snapshot.revision)
        } else {
            control.wall_surface_open(&binding, &id)
        };
        let state = bytes
            .map_err(|_| "Art视图读取失败")
            .and_then(|bytes| {
                State::parse(&bytes, &binding, &id, old.map(|e| &*e.state))
                    .map_err(|_| "Art视图校验失败")
            })
            .inspect_err(|_| {
                // 包括200但无可信身份的回复：不能推定open未创建并在两秒后再发。
                if old.is_none() {
                    shared.stop();
                }
            })?;
        let opening = old.is_none();
        if let Some(late) = admit(&mut self.entries, id.clone(), state, shared, job) {
            // 既有view的迟到state只丢弃；新open即使迟到也必须收回已验证的ephemeral view。
            if opening && control.wall_surface_close(&late.view).is_err() {
                shared.stop();
            }
            return Ok(());
        }
        if self
            .entries
            .values()
            .map(|e| e.state.snapshot.nodes)
            .sum::<usize>()
            > 512
        {
            return Err("可见Art节点预算不足");
        }
        let wanted: BTreeSet<_> = self
            .entries
            .values()
            .flat_map(|e| e.state.snapshot.dimensions.keys().cloned())
            .collect();
        if wanted.len() > 16 {
            return Err("可见Art图片数量超限");
        }
        self.images.retain(|id, _| wanted.contains(id));
        self.prepare()?;
        self.dispatch(&id, shared, job, false);
        Ok(())
    }
    fn prepare(&mut self) -> Result<(), &'static str> {
        for entry in self.entries.values_mut().filter(|e| e.ready.is_none()) {
            if entry
                .state
                .snapshot
                .dimensions
                .keys()
                .any(|id| !self.images.contains_key(id))
            {
                continue;
            }
            if entry
                .state
                .snapshot
                .dimensions
                .iter()
                .any(|(id, dims)| !dimensions_match(dims, &self.images[id]))
            {
                return Err("Art图片尺寸不匹配");
            }
            let images = entry
                .state
                .snapshot
                .images
                .iter()
                .map(|(node, resource)| {
                    self.images
                        .get(resource)
                        .cloned()
                        .map(|image| (node.clone(), image))
                        .ok_or("Art图片未就绪")
                })
                .collect::<Result<_, _>>()?;
            entry.ready = Some(Arc::new(Art {
                view: entry.state.view.clone(),
                generation: entry.state.generation,
                snapshot: entry.state.snapshot.clone(),
                size: entry.state.size,
                images,
            }));
        }
        Ok(())
    }
}
#[path = "tile_surface_owner_actions.rs"]
mod actions;
#[cfg(test)]
#[path = "tile_surface_owner_tests.rs"]
mod tests;
impl Drop for Views<'_> {
    fn drop(&mut self) {
        if let Ok(control) = self.lease.control(self.client) {
            for entry in self.entries.values() {
                let _ = control.wall_surface_close(&entry.state.view);
            }
        }
        // 撤权禁止借新凭据清理；原presenter disconnect/服务TTL回收view，绝不删除源instance。
    }
}
