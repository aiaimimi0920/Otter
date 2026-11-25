//! v2 accept write-ahead records use original session atomic-write/CAS/recovery file mechanisms; lost replies/cancellation preserve association identity.
use super::*;
use crate::pixels::Pixels;
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};
const LIMIT: usize = 4 * 1024 * 1024;
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Intent {
    pub envelope: Envelope,
    pub unit: String,
    pub scope: crate::session_projection_scope::Scope,
    pub loom_origin: String,
    pub device: String,
}
impl Intent {
    pub fn validate(&self) -> Result<(), Error> {
        self.envelope.validate()?;
        if crate::workspace_id::UnitId::parse(&self.unit).is_none()
            || !envelope::origin(&self.loom_origin)
            || !envelope::device(&self.device)
            || self.envelope.source_device() == self.device
        {
            return Err(Error::Input);
        }
        Ok(())
    }
    pub fn matches(&self, binding: &scene::Binding) -> bool {
        self.envelope == *binding.envelope()
            && self.unit == binding.unit()
            && self.scope == binding.scope()
            && self.loom_origin == binding.origin()
            && self.device == binding.device()
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema: u32,
    entries: Vec<Intent>,
}
pub struct Store {
    path: PathBuf,
    original: Option<Pixels>,
    entries: Vec<Intent>,
    _lock: File,
    _directories: Vec<File>,
}
impl Store {
    /// Open only on the single-slot worker; other processes sharing the profile cannot concurrently overwrite write-ahead records.
    pub fn open(root: &Path, cancel: &AtomicBool) -> Result<Self, Error> {
        let directories = crate::profile_directory::pin(root).map_err(|_| Error::Http)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join("projection-account.lock"))
            .map_err(|_| Error::Http)?;
        regular(&lock)?;
        let path = root.join("projection-account.json");
        let mut original = read(&path)?;
        if original.is_none() {
            if let Some(recovery) =
                crate::session_file_recovery::find(&path, cancel).map_err(|_| Error::Http)?
            {
                let bytes = read(&recovery)?.ok_or(Error::Response)?;
                decode(&bytes)?;
                crate::session_file_write::Staged::prepare_bytes(&path, bytes.clone(), cancel)
                    .map_err(|_| Error::Http)?
                    .commit_new(cancel)
                    .map_err(|_| Error::Http)?;
                original = Some(bytes);
            }
        }
        let entries = original
            .as_ref()
            .map(|bytes| decode(bytes))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            path,
            original,
            entries,
            _lock: lock,
            _directories: directories,
        })
    }
    pub fn entries(&self) -> &[Intent] {
        &self.entries
    }
    pub fn begin(&mut self, intent: Intent, cancel: &AtomicBool) -> Result<(), Error> {
        intent.validate()?;
        if let Some(old) = self
            .entries
            .iter()
            .find(|old| old.envelope == intent.envelope || old.unit == intent.unit)
        {
            if old != &intent {
                return Err(Error::Identity);
            }
            self.baseline(cancel)?;
            return Ok(());
        }
        if self.entries.len() >= crate::workspace::MAX_UNITS {
            return Err(Error::Budget);
        }
        let mut entries = self.entries.clone();
        entries.push(intent);
        self.persist(entries, cancel)
    }
    /// Retire only after proving never accepted or confirming original Unit unlink; network failure/window close cannot clear records.
    pub fn retire(&mut self, intent: &Intent, cancel: &AtomicBool) -> Result<(), Error> {
        let entries = self
            .entries
            .iter()
            .filter(|entry| *entry != intent)
            .cloned()
            .collect();
        self.persist(entries, cancel)
    }
    fn baseline(&self, cancel: &AtomicBool) -> Result<(), Error> {
        if let Some(expected) = &self.original {
            crate::session_file_guard::Guard::open(&self.path, expected, cancel)
                .map_err(|_| Error::Response)?;
        } else if self.path.try_exists().map_err(|_| Error::Http)? {
            return Err(Error::Response);
        }
        Ok(())
    }
    fn persist(&mut self, entries: Vec<Intent>, cancel: &AtomicBool) -> Result<(), Error> {
        let bytes = crate::session_write::encode_bounded(
            &Ledger {
                schema: 1,
                entries: entries.clone(),
            },
            cancel,
        )
        .map_err(|_| Error::Budget)?;
        if bytes.len() > LIMIT {
            return Err(Error::Budget);
        }
        self.baseline(cancel)?;
        let retained = if let Some(expected) = &self.original {
            crate::session_file_replace::Replacement::prepare_bytes(
                &self.path,
                expected,
                bytes.clone(),
                cancel,
            )
            .map_err(|_| Error::Http)?
            .commit(cancel)
            .map_err(|_| Error::Http)?
            .retained_recovery
            .is_some()
        } else {
            crate::session_file_write::Staged::prepare_bytes(&self.path, bytes.clone(), cancel)
                .map_err(|_| Error::Http)?
                .commit_new(cancel)
                .map_err(|_| Error::Http)?;
            false
        };
        self.original = Some(bytes);
        self.entries = entries;
        if retained {
            Err(Error::Http)
        } else {
            Ok(())
        }
    }
}
fn decode(bytes: &[u8]) -> Result<Vec<Intent>, Error> {
    if bytes.len() > LIMIT {
        return Err(Error::Budget);
    }
    let ledger: Ledger = serde_json::from_slice(bytes).map_err(|_| Error::Response)?;
    if ledger.schema != 1 || ledger.entries.len() > crate::workspace::MAX_UNITS {
        return Err(Error::Response);
    }
    for (index, entry) in ledger.entries.iter().enumerate() {
        entry.validate()?;
        if ledger.entries[..index]
            .iter()
            .any(|old| old.envelope == entry.envelope || old.unit == entry.unit)
        {
            return Err(Error::Response);
        }
    }
    Ok(ledger.entries)
}
fn regular(file: &File) -> Result<(), Error> {
    let metadata = file.metadata().map_err(|_| Error::Http)?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::Http);
    }
    Ok(())
}
fn read(path: &Path) -> Result<Option<Pixels>, Error> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Http),
    };
    regular(&file)?;
    let size = usize::try_from(file.metadata().map_err(|_| Error::Http)?.len())
        .map_err(|_| Error::Budget)?;
    if size > LIMIT {
        return Err(Error::Budget);
    }
    let mut lease = crate::pixels::reserve(size + 1).ok_or(Error::Budget)?;
    let mut bytes = Vec::with_capacity(size + 1);
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::Budget);
    }
    file.take((size + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Http)?;
    if bytes.len() != size {
        return Err(Error::Response);
    }
    Ok(Some(Pixels::from_reserved(bytes, lease)))
}
