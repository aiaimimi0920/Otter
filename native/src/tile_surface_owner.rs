//! At most four ephemeral views within the exclusive serial Art request thread; pass pure data to UI while HTTP/resource owners remain thread-local.
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
    // Download one image per step without extra sampling/resource tasks; original Live continues consumption on its independent Assets thread.
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
// Hold the Shared lock when admitting scenes and clearing old candidates; late state during freeze/generation changes cannot replace retained scenes.
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
        s.unavailable(job.generation, &job.key, "tile-art-preparing");
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
    // Content addressing shares only within this job/view authorization scope; generation changes release everything without inheriting old authorization.
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
            return Err("tile-art-count-limit");
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
            .map_err(|_| "tile-authorization-invalid")?;
        if let Task::Image(id, resource) = task {
            let entry = &self.entries[&id];
            let image = control
                .wall_surface_image(&entry.state.view, &resource)
                .map_err(|_| "tile-art-image-read-failed")?;
            if !super::tile_assets::live::current(shared, job) {
                return Ok(());
            }
            if !dimensions_match(&entry.state.snapshot.dimensions[&resource], &image) {
                return Err("tile-art-image-dimensions");
            }
            let pixels = u64::from(image.width) * u64::from(image.height);
            if other_pixels + self.pixels() + pixels > crate::tile_wire::MAX_PIXELS {
                return Err("tile-content-pixel-budget");
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
            .map_err(|_| "tile-art-view-read-failed")
            .and_then(|bytes| {
                State::parse(&bytes, &binding, &id, old.map(|e| &*e.state))
                    .map_err(|_| "tile-art-view-invalid")
            })
            .inspect_err(|_| {
                // This includes 200 replies lacking trusted identity: never infer that open created nothing and send again two seconds later.
                if old.is_none() {
                    shared.stop();
                }
            })?;
        let opening = old.is_none();
        if let Some(late) = admit(&mut self.entries, id.clone(), state, shared, job) {
            // Discard late state for existing views; new opens must reclaim verified ephemeral views even when late.
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
            return Err("tile-art-node-budget");
        }
        let wanted: BTreeSet<_> = self
            .entries
            .values()
            .flat_map(|e| e.state.snapshot.dimensions.keys().cloned())
            .collect();
        if wanted.len() > 16 {
            return Err("tile-art-image-count-limit");
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
                return Err("tile-art-image-dimensions");
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
                        .ok_or("tile-art-image-not-ready")
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_surface_owner_tests.rs"]
mod tests;
impl Drop for Views<'_> {
    fn drop(&mut self) {
        if let Ok(control) = self.lease.control(self.client) {
            for entry in self.entries.values() {
                let _ = control.wall_surface_close(&entry.state.view);
            }
        }
        // Revocation prohibits cleanup with new credentials; original presenter disconnect/service TTL reclaims views without ever deleting source instances.
    }
}
