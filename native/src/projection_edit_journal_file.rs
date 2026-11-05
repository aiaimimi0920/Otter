//! 编辑日志复用原session原子写/完整CAS/恢复副本，不另造磁盘事务或清理策略。
use super::*;
use std::{
    fs::OpenOptions,
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};
pub(super) fn open(root: &Path, cancel: &AtomicBool) -> Result<Store, Error> {
    check(cancel)?;
    let directories = crate::profile_directory::pin(root).map_err(|_| Error::Path)?;
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(root.join("projection-edit.lock"))
        .map_err(|_| Error::Lock)?;
    regular(&lease, cancel)?;
    let path = root.join("projection-edit.json");
    let mut original = read(&path, cancel)?;
    if original.is_none() {
        if let Some(recovery) = crate::session_file_recovery::find(&path, cancel)
            .map_err(|_| cancelled(cancel, Error::Read))?
        {
            let bytes = read(&recovery, cancel)?.ok_or(Error::Read)?;
            Ledger::decode(&bytes)?;
            crate::session_file_write::Staged::prepare_bytes(&path, bytes.clone(), cancel)
                .map_err(|_| cancelled(cancel, Error::Write))?
                .commit_new(cancel)
                .map_err(|_| cancelled(cancel, Error::Write))?;
            original = Some(bytes);
        }
    }
    let entries = match &original {
        Some(bytes) => Ledger::decode(bytes)?.entries,
        None => Vec::new(),
    };
    check(cancel)?;
    Ok(Store(Arc::new(Inner {
        path,
        state: Mutex::new(State {
            original,
            entries,
            retained: false,
        }),
        active: AtomicBool::new(false),
        _lease: lease,
        _directories: directories,
    })))
}
impl Store {
    pub(super) fn baseline(&self, state: &State, cancel: &AtomicBool) -> Result<(), Error> {
        check(cancel)?;
        if state.retained {
            return Err(Error::RecoveryRetained);
        }
        if let Some(expected) = &state.original {
            let _guard = crate::session_file_guard::Guard::open(&self.0.path, expected, cancel)
                .map_err(|_| cancelled(cancel, Error::Changed))?;
        } else if self
            .0
            .path
            .try_exists()
            .map_err(|_| cancelled(cancel, Error::Changed))?
            || crate::session_file_recovery::find(&self.0.path, cancel)
                .map_err(|_| cancelled(cancel, Error::Changed))?
                .is_some()
        {
            return Err(Error::Changed);
        }
        check(cancel)
    }
    pub(super) fn persist(
        &self,
        state: &mut State,
        entries: Vec<Entry>,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        check(cancel)?;
        let ledger = Ledger::new(entries);
        ledger.validate()?;
        let bytes = crate::session_write::encode_bounded(&ledger, cancel).map_err(|_| {
            if cancel.load(Ordering::Acquire) {
                Error::Stopped
            } else {
                Error::Limit
            }
        })?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        self.baseline(state, cancel)?;
        let retained = if let Some(expected) = &state.original {
            crate::session_file_replace::Replacement::prepare_bytes(
                &self.0.path,
                expected,
                bytes.clone(),
                cancel,
            )
            .map_err(|_| cancelled(cancel, Error::Changed))?
            .commit(cancel)
            .map_err(|_| cancelled(cancel, Error::Write))?
            .retained_recovery
            .is_some()
        } else {
            crate::session_file_write::Staged::prepare_bytes(&self.0.path, bytes.clone(), cancel)
                .map_err(|_| cancelled(cancel, Error::Write))?
                .commit_new(cancel)
                .map_err(|_| cancelled(cancel, Error::Changed))?;
            false
        };
        // 即使退休恢复副本失败，真实已发布的字节仍成为当前基线，但禁止继续网络副作用。
        state.original = Some(bytes);
        state.entries = ledger.entries;
        state.retained = retained;
        if retained {
            Err(Error::RecoveryRetained)
        } else {
            Ok(())
        }
    }
}
fn cancelled(cancel: &AtomicBool, error: Error) -> Error {
    if cancel.load(Ordering::Acquire) {
        Error::Stopped
    } else {
        error
    }
}
fn regular(file: &File, cancel: &AtomicBool) -> Result<(), Error> {
    let meta = file
        .metadata()
        .map_err(|_| cancelled(cancel, Error::Read))?;
    if !meta.is_file() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::Read);
    }
    Ok(())
}
fn read(path: &Path, cancel: &AtomicBool) -> Result<Option<Pixels>, Error> {
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Read),
    };
    regular(&file, cancel)?;
    let length = usize::try_from(
        file.metadata()
            .map_err(|_| cancelled(cancel, Error::Read))?
            .len(),
    )
    .map_err(|_| Error::Limit)?;
    if length > MAX_BYTES {
        return Err(Error::Limit);
    }
    let mut lease = crate::pixels::reserve(length + 1).ok_or(Error::Limit)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length + 1)
        .map_err(|_| Error::Limit)?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::Limit);
    }
    bytes.resize(length + 1, 0);
    let mut used = 0;
    while used < bytes.len() {
        check(cancel)?;
        let end = (used + 48 * 1024).min(bytes.len());
        let count = file
            .read(&mut bytes[used..end])
            .map_err(|_| cancelled(cancel, Error::Read))?;
        if count == 0 {
            break;
        }
        used += count;
    }
    if used != length {
        return Err(Error::Changed);
    }
    bytes.truncate(used);
    check(cancel)?;
    Ok(Some(Pixels::from_reserved(bytes, lease)))
}
