//! Bounded request queues per current scene; no automatic retries after start, rebuilding only on binding/version changes.
use super::*;
use crate::surface_resource_runtime::request::{
    inline::{self, Inline},
    Key, Request,
};
use crate::{
    surface_live_binding::Binding,
    surface_scene::{Kind, Node, Scene},
};
use std::collections::{BTreeSet, VecDeque};

#[derive(Default)]
pub(in crate::desktop::surface_authorized_window) struct Requests {
    identity: Option<(Binding, u64)>,
    pending: VecDeque<Key>,
    error: Option<String>,
}
fn references(current: &MountedSurface) -> std::result::Result<VecDeque<Key>, ImageError> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        scene: Box<serde_json::value::RawValue>,
    }
    let envelope: Envelope = serde_json::from_str(current.snapshot.get())
        .map_err(|_| ImageError::Resource(ResourceError::Identity))?;
    let scene = Scene::parse(envelope.scene.get().as_bytes())
        .map_err(|_| ImageError::Resource(ResourceError::Identity))?;
    fn visit(
        node: &Node,
        seen: &mut BTreeSet<Key>,
        ids: &mut VecDeque<Key>,
    ) -> std::result::Result<(), ImageError> {
        if !node.visible() {
            return Ok(());
        }
        if node.kind() == Kind::Image {
            if let Some(id) = inline::key(node) {
                if seen.insert(id.clone()) {
                    if ids.len() == 64 {
                        return Err(ImageError::Resource(ResourceError::Limit));
                    }
                    ids.push_back(id);
                }
            }
            return Ok(());
        }
        for child in node.visible_children() {
            visit(child, seen, ids)?;
        }
        Ok(())
    }
    let mut ids = VecDeque::new();
    visit(scene.root(), &mut BTreeSet::new(), &mut ids)?;
    Ok(ids)
}
impl AuthorizedWindow {
    pub(crate) fn next_image_request(
        &mut self,
        now: u64,
    ) -> std::result::Result<Option<Request>, ImageError> {
        // mounted -> active changes the complete Binding; automatic tasks do not prefetch before activation.
        if self.javascript.is_some()
            || !self.is_alive()
            || !self.lifecycle().is_some_and(Owner::confirmed_active)
        {
            return Ok(None);
        }
        {
            let requests = self.image_requests.borrow();
            if requests.pending.is_empty()
                && requests
                    .identity
                    .as_ref()
                    .is_some_and(|(binding, revision)| {
                        binding == self.binding() && *revision == self.snapshot_revision
                    })
            {
                // Common message hot paths neither copy nor parse whole snapshots; main-loop refresh still handles window revocation.
                return Ok(None);
            }
        }
        let current = self.resource_mount(now)?;
        let key = (current.binding.clone(), self.snapshot_revision);
        if self.image_requests.borrow().identity.as_ref() != Some(&key) {
            // Record this version first; even parsing failures must not repeat work/errors on every message-loop round.
            *self.image_requests.get_mut() = Requests {
                identity: Some(key),
                ..Default::default()
            };
            self.image_requests.get_mut().pending = references(&current)?;
        }
        let Some(id) = self.image_requests.get_mut().pending.front().cloned() else {
            return Ok(None);
        };
        let request = match id {
            Key::Resource(id) => current.image_resource(&id, now).map(Request::Resource),
            Key::Node(id) => Inline::from_mount(&current, &id).map(Request::Inline),
        };
        match request {
            Ok(request) => Ok(Some(request)),
            Err(error) => {
                self.finish_image_request();
                Err(ImageError::Resource(error))
            }
        }
    }
    pub(crate) fn finish_image_request(&mut self) {
        self.image_requests.get_mut().pending.pop_front();
    }
    pub(crate) fn record_image_error(&mut self, message: &str) {
        self.image_requests.get_mut().error = Some(message.chars().take(512).collect());
    }
    pub(crate) fn image_error(&self) -> Option<String> {
        self.image_requests.borrow().error.clone()
    }
}
