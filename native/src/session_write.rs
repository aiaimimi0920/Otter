//! Whole-session memory candidates: merge by identity and preserve unknown root fields; disk committers must validate expected_revision.
#[path = "session_archive_write.rs"]
pub mod archive;
use crate::{
    session_document::{decode, MAX_SESSION_BYTES},
    workspace_session::{Sticker, Topology},
};
use serde_json::{json, value::RawValue, Map, Value};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        LazyLock, Mutex, Weak,
    },
};

type VerifiedSlot = (Weak<crate::pixels::Storage>, u64, u64);
static VERIFIED_CANDIDATES: LazyLock<Mutex<VecDeque<VerifiedSlot>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(8)));

fn record_verified(bytes: &crate::pixels::Pixels, expected_revision: u64, revision: u64) {
    if let Ok(mut guard) = VERIFIED_CANDIDATES.lock() {
        guard.retain(|(weak, _, _)| weak.strong_count() > 0);
        while guard.len() >= 8 {
            guard.pop_front();
        }
        guard.push_back((bytes.downgrade(), expected_revision, revision));
    }
}

pub(crate) fn is_verified(candidate: &Candidate) -> bool {
    let probe = candidate.bytes.downgrade();
    VERIFIED_CANDIDATES.lock().is_ok_and(|guard| {
        guard.iter().any(|(weak, expected, rev)| {
            Weak::ptr_eq(weak, &probe)
                && *expected == candidate.expected_revision
                && *rev == candidate.revision
        })
    })
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum RootEntry<'a> {
    Raw(&'a RawValue),
    Windows(Vec<WindowCandidate>),
    Value(Value),
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Source,
    Topology,
    Window,
    Revision,
    Budget,
}
pub struct Candidate {
    pub bytes: crate::pixels::Pixels,
    pub expected_revision: u64,
    pub revision: u64,
}

/// Deliver window fields with budgets for newly encoded strings; release fields before leases.
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct WindowCandidate {
    fields: Map<String, Value>,
    #[serde(skip)]
    leases: Vec<crate::byte_budget::Lease>,
}
impl WindowCandidate {
    #[cfg(windows)]
    pub(crate) fn write_projection_edit(
        &mut self,
        snapshot: Option<&crate::projection_edit::workspace::Snapshot>,
    ) -> Result<(), Error> {
        self.fields.remove(crate::projection_edit::workspace::FIELD);
        if let Some(snapshot) = snapshot {
            snapshot
                .install(&mut self.fields)
                .map_err(|_| Error::Window)?;
        }
        Ok(())
    }
    pub fn new(fields: Map<String, Value>) -> Self {
        Self {
            fields,
            leases: Vec::new(),
        }
    }
    /// Move the same string allocation; do not release its budget before handing it to serde.
    pub fn insert_encoded(&mut self, key: &str, value: crate::loom_base64::Encoded) {
        let (text, lease) = value.into_parts();
        self.fields.insert(key.into(), Value::String(text));
        self.leases.push(lease);
    }
    /// Merge image and scalar ports into one object, retaining string budgets until the entire candidate is released.
    pub fn insert_encoded_output(
        &mut self,
        key: &str,
        value: crate::loom_base64::Encoded,
    ) -> Result<(), Error> {
        let Some(Value::Object(outputs)) = self.fields.get_mut("outputs") else {
            return Err(Error::Window);
        };
        if key.is_empty()
            || key.len() > 256
            || key.chars().any(char::is_control)
            || outputs.contains_key(key)
            || outputs.len() >= 128
        {
            return Err(Error::Window);
        }
        self.leases.try_reserve(1).map_err(|_| Error::Budget)?;
        let (text, lease) = value.into_parts();
        outputs.insert(key.into(), Value::String(text));
        self.leases.push(lease);
        Ok(())
    }
}

/// build consumes old window objects without copying image strings; new objects receive an empty Map and must supply complete window content.
/// Callbacks construct memory only, with no published side effects. Errors, cancellation, or readback failure deliver no partial candidate.
pub fn assemble(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    mut build: impl FnMut(&Sticker, Map<String, Value>) -> Result<Map<String, Value>, Error>,
) -> Result<Candidate, Error> {
    assemble_owned(original, topology, cancel, |sticker, old| {
        build(sticker, old).map(WindowCandidate::new)
    })
}

/// Budgeted resource-installation entry point; retain all window leases until the complete JSON tree is released.
pub fn assemble_owned(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
) -> Result<Candidate, Error> {
    assemble_owned_with_space(original, topology, cancel, None, build)
}

/// Native saves carry explicit coordinate space; old callers neither guess nor rewrite source space.
pub fn assemble_owned_with_space(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    space: Option<crate::session_placement::Space>,
    build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
) -> Result<Candidate, Error> {
    assemble_owned_at_revision(original, topology, cancel, space, None, None, build)
}

/// Independent recovery checkpoints may have their own disk revisions; the same assembly/budget/readback owner still encodes once.
pub(crate) fn assemble_owned_at_revision(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    space: Option<crate::session_placement::Space>,
    disk_revision: Option<u64>,
    projection_scope: Option<crate::session_projection_scope::Scope>,
    build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
) -> Result<Candidate, Error> {
    assemble_with_reserve(
        original,
        topology,
        cancel,
        space,
        disk_revision,
        projection_scope,
        build,
        crate::pixels::reserve,
    )
}

fn assemble_with_reserve(
    original: &[u8],
    topology: &Topology,
    cancel: &AtomicBool,
    space: Option<crate::session_placement::Space>,
    disk_revision: Option<u64>,
    projection_scope: Option<crate::session_projection_scope::Scope>,
    mut build: impl FnMut(&Sticker, Map<String, Value>) -> Result<WindowCandidate, Error>,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<Candidate, Error> {
    check(cancel)?;
    let source = decode(original).map_err(|_| Error::Source)?;
    let expected_revision = disk_revision.unwrap_or(source.revision);
    let revision = expected_revision.checked_add(1).ok_or(Error::Revision)?;
    topology.restore().map_err(|_| Error::Topology)?;
    drop(source);
    // Borrow source content through top-level RawValue; unchanged frozen galleries/large extension fields pass through allocation-free.
    // JSON field names may contain escapes; own only these small keys while values borrow source content without copying large resources.
    let mut raw_root: BTreeMap<String, &RawValue> =
        serde_json::from_slice(original).map_err(|_| Error::Source)?;
    let parse_entry = |raw: Option<&RawValue>| -> Result<Option<Value>, Error> {
        raw.map(|v| serde_json::from_str(v.get()).map_err(|_| Error::Source))
            .transpose()
    };
    let mut windows = indexed(parse_entry(raw_root.remove("stickers"))?)?;
    let old_links = indexed(parse_entry(raw_root.remove("links"))?)?;
    let old_groups = indexed(parse_entry(raw_root.remove("groups"))?)?;
    raw_root.remove("documentSchemaVersion");
    raw_root.remove("documentRevision");

    let mut meta = Map::new();
    if projection_scope.is_some() {
        if let Some(existing) = parse_entry(raw_root.remove("otterNativeProjectionScope"))? {
            meta.insert("otterNativeProjectionScope".into(), existing);
        }
    }
    if space.is_some() {
        if let Some(existing) = parse_entry(raw_root.remove("otterNativeCoordinateSpace"))? {
            meta.insert("otterNativeCoordinateSpace".into(), existing);
        }
    }
    if let Some(scope) = projection_scope {
        scope.install(&mut meta)?;
    }
    if let Some(space) = space {
        crate::session_coordinates::install(&mut meta, space).map_err(|_| Error::Source)?;
    }

    let mut used = 0usize;
    let mut built_windows = Vec::with_capacity(topology.stickers.len());
    for sticker in &topology.stickers {
        check(cancel)?;
        let old = windows.remove(&sticker.id).unwrap_or_default();
        let mut window = build(sticker, old)?;
        // Validated topology owns identities/groups; callbacks cannot secretly introduce another object or dangling group.
        if window
            .fields
            .get("id")
            .is_some_and(|id| id.as_str() != Some(&sticker.id))
        {
            return Err(Error::Window);
        }
        window.fields.insert("id".into(), sticker.id.clone().into());
        match &sticker.group_id {
            Some(id) => {
                window.fields.insert("groupId".into(), id.clone().into());
            }
            None => {
                window.fields.remove("groupId");
            }
        }
        used = used
            .checked_add(measure(&window, cancel)?)
            .ok_or(Error::Budget)?;
        if used > MAX_SESSION_BYTES {
            return Err(Error::Budget);
        }
        built_windows.push(window);
    }
    drop(windows);
    let mut root: BTreeMap<String, RootEntry<'_>> = raw_root
        .into_iter()
        .map(|(k, v)| (k, RootEntry::Raw(v)))
        .collect();
    for (k, v) in meta {
        root.insert(k, RootEntry::Value(v));
    }
    root.insert("stickers".into(), RootEntry::Windows(built_windows));
    root.insert(
        "links".into(),
        RootEntry::Value(merge_records(old_links, &topology.links, cancel)?.into()),
    );
    root.insert(
        "groups".into(),
        RootEntry::Value(merge_records(old_groups, &topology.groups, cancel)?.into()),
    );
    root.insert("documentSchemaVersion".into(), RootEntry::Value(json!(1)));
    root.insert("documentRevision".into(), RootEntry::Value(json!(revision)));
    check(cancel)?;
    // Retain no separate complete set of encoded-window buffers; shared budget needs only final output and existing window-resource leases.
    let length = match measure(&root, cancel) {
        Ok(length) => length,
        Err(Error::Budget) => {
            // Valid legacy files may exactly meet the limit; compact whitespace in unknown fields only on this slow path, without rerunning build.
            for entry in root.values_mut() {
                check(cancel)?;
                if let RootEntry::Raw(raw) = entry {
                    *entry = RootEntry::Value(
                        serde_json::from_str(raw.get()).map_err(|_| Error::Source)?,
                    );
                }
            }
            measure(&root, cancel)?
        }
        Err(error) => return Err(error),
    };
    let mut output = Sink::new(length, cancel, reserve)?;
    serde_json::to_writer(&mut output, &root).map_err(|_| failure(cancel))?;
    drop(root);
    check(cancel)?;
    let checked = decode(&output.bytes).map_err(|_| Error::Window)?;
    if checked.revision != revision || Topology::capture(&checked.workspace) != *topology {
        return Err(Error::Topology);
    }
    check(cancel)?;
    let bytes = crate::pixels::Pixels::from_reserved(output.bytes, output.lease);
    record_verified(&bytes, expected_revision, revision);
    Ok(Candidate {
        bytes,
        expected_revision,
        revision,
    })
}
/// Shared bounded serialization; output holds the shared pixel budget, and failure delivers no partial bytes.
pub(crate) fn encode_bounded(
    value: &impl serde::Serialize,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, Error> {
    check(cancel)?;
    let length = measure(value, cancel)?;
    let mut output = Sink::new(length, cancel, crate::pixels::reserve)?;
    serde_json::to_writer(&mut output, value).map_err(|_| failure(cancel))?;
    check(cancel)?;
    Ok(crate::pixels::Pixels::from_reserved(
        output.bytes,
        output.lease,
    ))
}

fn indexed(value: Option<Value>) -> Result<HashMap<String, Map<String, Value>>, Error> {
    let items = match value {
        None => Vec::new(),
        Some(Value::Array(items)) => items,
        _ => return Err(Error::Source),
    };
    let mut map = HashMap::with_capacity(items.len());
    for item in items {
        let Value::Object(item) = item else {
            return Err(Error::Source);
        };
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::Source)?
            .to_owned();
        if map.insert(id, item).is_some() {
            return Err(Error::Source);
        }
    }
    Ok(map)
}
fn merge_records<T: serde::Serialize>(
    mut previous: HashMap<String, Map<String, Value>>,
    records: &[T],
    cancel: &AtomicBool,
) -> Result<Vec<Value>, Error> {
    let mut values = Vec::with_capacity(records.len());
    for record in records {
        check(cancel)?;
        let Value::Object(fields) = serde_json::to_value(record).map_err(|_| Error::Topology)?
        else {
            return Err(Error::Topology);
        };
        let id = fields
            .get("id")
            .and_then(Value::as_str)
            .ok_or(Error::Topology)?;
        let mut value = previous.remove(id).unwrap_or_default();
        value.extend(fields);
        values.push(value.into());
    }
    Ok(values)
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn failure(cancel: &AtomicBool) -> Error {
    if cancel.load(Ordering::Acquire) {
        Error::Cancelled
    } else {
        Error::Budget
    }
}
fn measure(value: &impl serde::Serialize, cancel: &AtomicBool) -> Result<usize, Error> {
    struct Count<'a> {
        size: usize,
        cancel: &'a AtomicBool,
    }
    impl Write for Count<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.cancel.load(Ordering::Acquire)
                || bytes.len() > MAX_SESSION_BYTES.saturating_sub(self.size)
            {
                return Err(std::io::Error::other(
                    "Session candidate budget or cancellation",
                ));
            }
            self.size += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count { size: 0, cancel };
    serde_json::to_writer(&mut count, value).map_err(|_| failure(cancel))?;
    Ok(count.size)
}
struct Sink<'a> {
    bytes: Vec<u8>,
    lease: crate::byte_budget::Lease,
    cancel: &'a AtomicBool,
}
impl<'a> Sink<'a> {
    fn new(
        length: usize,
        cancel: &'a AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<Self, Error> {
        check(cancel)?;
        if length > MAX_SESSION_BYTES {
            return Err(Error::Budget);
        }
        let mut lease = reserve(length).ok_or(Error::Budget)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| Error::Budget)?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Budget);
        }
        check(cancel)?;
        Ok(Self {
            bytes,
            lease,
            cancel,
        })
    }
}
impl Write for Sink<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.cancel.load(Ordering::Acquire)
            || bytes.len() > MAX_SESSION_BYTES.saturating_sub(self.bytes.len())
            || bytes.len() > self.bytes.capacity().saturating_sub(self.bytes.len())
        {
            return Err(std::io::Error::other(
                "Session candidate budget or cancellation",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_write_tests.rs"]
mod tests;
