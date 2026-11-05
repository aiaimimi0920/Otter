//! 独占发送日志复用 session 文件事务；完整基线冲突、恢复副本、取消都不降级为空。
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
        .open(root.join("projection-sender.lock"))
        .map_err(|_| Error::Lock)?;
    regular(&lease).map_err(|_| Error::Lock)?;
    let path = root.join("projection-sender.json");
    let mut original = read(&path, cancel)?;
    if original.is_none() {
        if let Some(recovery) =
            crate::session_file_recovery::find(&path, cancel).map_err(|_| Error::Read)?
        {
            let bytes = read(&recovery, cancel)?.ok_or(Error::Read)?;
            // 完整格式/PNG 验证先于无覆盖重建；旧恢复副本始终保留。
            Document::decode(&bytes, cancel)?;
            crate::session_file_write::Staged::prepare_bytes(&path, bytes.clone(), cancel)
                .map_err(|_| Error::Write)?
                .commit_new(cancel)
                .map_err(|_| Error::Write)?;
            original = Some(bytes);
        }
    }
    let entries = match &original {
        Some(bytes) => Document::decode(bytes, cancel)?,
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
        activity: activity::Registry::default(),
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
                .map_err(|_| Error::Changed)?;
        } else if self.0.path.try_exists().map_err(|_| Error::Changed)?
            || crate::session_file_recovery::find(&self.0.path, cancel)
                .map_err(|_| Error::Changed)?
                .is_some()
        {
            return Err(Error::Changed);
        }
        check(cancel)
    }
    pub(super) fn persist(
        &self,
        state: &mut State,
        entries: Vec<Record>,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        check(cancel)?;
        // session encoder 的 64 MiB 上限不替代本日志 48 MiB；计数门先于目的分配。
        let document = Document::new(&entries);
        struct Count<'a>(usize, &'a AtomicBool);
        impl std::io::Write for Count<'_> {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self.1.load(Ordering::Acquire) || bytes.len() > MAX_BYTES.saturating_sub(self.0)
                {
                    return Err(std::io::Error::other("Sender journal limit"));
                }
                self.0 += bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer(&mut Count(0, cancel), &document).map_err(|_| {
            if cancel.load(Ordering::Acquire) {
                Error::Stopped
            } else {
                Error::Limit
            }
        })?;
        check(cancel)?;
        let bytes = crate::session_write::encode_bounded(&document, cancel).map_err(|_| {
            if cancel.load(Ordering::Acquire) {
                Error::Stopped
            } else {
                Error::Limit
            }
        })?;
        self.baseline(state, cancel)?;
        let retained = if let Some(expected) = &state.original {
            crate::session_file_replace::Replacement::prepare_bytes(
                &self.0.path,
                expected,
                bytes.clone(),
                cancel,
            )
            .map_err(|_| Error::Changed)?
            .commit(cancel)
            .map_err(|_| Error::Write)?
            .retained_recovery
            .is_some()
        } else {
            crate::session_file_write::Staged::prepare_bytes(&self.0.path, bytes.clone(), cancel)
                .map_err(|_| Error::Write)?
                .commit_new(cancel)
                .map_err(|_| Error::Changed)?;
            false
        };
        state.published(bytes, entries, retained)
    }
}
impl State {
    pub(super) fn published(
        &mut self,
        bytes: Pixels,
        entries: Vec<Record>,
        retained: bool,
    ) -> Result<(), Error> {
        self.original = Some(bytes);
        self.entries = entries;
        self.retained = retained;
        if retained {
            Err(Error::RecoveryRetained)
        } else {
            Ok(())
        }
    }
}
fn regular(file: &File) -> Result<(), Error> {
    let meta = file.metadata().map_err(|_| Error::Read)?;
    if !meta.is_file() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::Read);
    }
    Ok(())
}
fn read(path: &Path, cancel: &AtomicBool) -> Result<Option<Pixels>, Error> {
    check(cancel)?;
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Read),
    };
    regular(&file)?;
    let length = usize::try_from(file.metadata().map_err(|_| Error::Read)?.len())
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
        let count = file.read(&mut bytes[used..end]).map_err(|_| Error::Read)?;
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
