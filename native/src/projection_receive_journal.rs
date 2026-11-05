//! 接受请求的写前队列；原文件 CAS/暂存/恢复共用 session 文件 owner，只确认解绑后退休。
use crate::{
    pixels::Pixels,
    projection_delivery::{Envelope, Invitation},
};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};
#[path = "projection_receive_activity.rs"]
mod activity;
#[path = "projection_receive_cleanup.rs"]
mod cleanup;
#[path = "projection_receive_record.rs"]
mod record;
pub(super) use record::intent_id as record_intent_id;
use record::{Document, Record};
pub use record::{MAX_BYTES, MAX_ENTRIES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Path,
    Lock,
    Read,
    Format,
    Limit,
    Changed,
    Write,
    /// 新队列已经提交，但旧恢复副本退休失败；不能伪报全未提交，也不继续发 accept。
    RecoveryRetained,
    Busy,
}
struct State {
    original: Option<Pixels>,
    entries: Vec<Record>,
}
impl State {
    fn published(
        &mut self,
        bytes: Pixels,
        document: Document,
        retained: bool,
    ) -> Result<(), Error> {
        self.original = Some(bytes);
        self.entries = document.entries;
        if retained {
            Err(Error::RecoveryRetained)
        } else {
            Ok(())
        }
    }
}
struct Inner {
    path: PathBuf,
    state: Mutex<State>,
    activity: activity::Registry,
    _lease: File,
    _directories: Vec<File>,
}
#[derive(Clone)]
pub struct Store(Arc<Inner>);
pub struct Ticket {
    store: Store,
    record: Record,
    permit: Option<activity::Permit>,
}
impl Store {
    /// 只在 worker 上执行；根目录由宿主显式给出，不在这里读取环境或发现身份。
    pub fn open(root: &Path) -> Result<Self, Error> {
        let directories = crate::profile_directory::pin(root).map_err(|_| Error::Path)?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join("projection-receive.lock"))
            .map_err(|_| Error::Lock)?;
        regular(&lease).map_err(|_| Error::Lock)?;
        let path = root.join("projection-receive.json");
        let mut original = read(&path)?;
        if original.is_none() {
            if let Some(recovery) =
                crate::session_file_recovery::find(&path, &AtomicBool::new(false))
                    .map_err(|_| Error::Read)?
            {
                let bytes = read(&recovery)?.ok_or(Error::Read)?;
                Document::decode(&bytes)?;
                // 唯一旧恢复副本只读保留；无覆盖重建规范路径，不能降级为空队列。
                crate::session_file_write::Staged::prepare_bytes(
                    &path,
                    bytes.clone(),
                    &AtomicBool::new(false),
                )
                .map_err(|_| Error::Write)?
                .commit_new(&AtomicBool::new(false))
                .map_err(|_| Error::Write)?;
                original = Some(bytes);
            }
        }
        let entries = match &original {
            Some(bytes) => Document::decode(bytes)?.entries,
            None => Vec::new(),
        };
        Ok(Self(Arc::new(Inner {
            path,
            state: Mutex::new(State { original, entries }),
            activity: activity::Registry::default(),
            _lease: lease,
            _directories: directories,
        })))
    }
    /// 幂等重试使用原意图的 expectedRevision/digest，不因收件箱更新而换请求身份。
    pub fn begin(&self, invitation: &Invitation, device: &str) -> Result<Ticket, Error> {
        self.begin_attempt(invitation, device)
            .map(|(ticket, _)| ticket)
    }
    /// 原子区分新建与复用；复用意图可能属于其他已保存工作区，不自动授予补偿资格。
    pub(crate) fn begin_attempt(
        &self,
        invitation: &Invitation,
        device: &str,
    ) -> Result<(Ticket, bool), Error> {
        let record = Record::new(invitation, device)?;
        let mut state = self.0.state.lock().map_err(|_| Error::Changed)?;
        if let Some(old) = state.entries.iter().find(|old| old.same_key(&record)) {
            if old.device != device
                || old.envelope != record.envelope
                || old.offline != record.offline
                || old.cleanup
            {
                return Err(Error::Changed);
            }
            // 重试也必须核完整磁盘基线，不能以旧内存状态绕过外部修改/删除。
            let expected = state.original.as_ref().ok_or(Error::Changed)?;
            let _guard = crate::session_file_guard::Guard::open(
                &self.0.path,
                expected,
                &AtomicBool::new(false),
            )
            .map_err(|_| Error::Changed)?;
            return Ok((
                Ticket {
                    store: self.clone(),
                    record: old.clone(),
                    permit: None,
                },
                false,
            ));
        }
        if state.entries.len() >= MAX_ENTRIES {
            return Err(Error::Limit);
        }
        let mut next = state.entries.clone();
        next.push(record.clone());
        self.persist(&mut state, next)?;
        Ok((
            Ticket {
                store: self.clone(),
                record,
                permit: None,
            },
            true,
        ))
    }
    /// 仅返回补偿候选，不执行 accept、不自动换授权或在新工作区发布内容。
    pub fn pending(&self) -> Result<Vec<Ticket>, Error> {
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        Ok(state
            .entries
            .iter()
            .map(|record| Ticket {
                store: self.clone(),
                record: record.clone(),
                permit: None,
            })
            .collect())
    }
    fn persist(&self, state: &mut State, next: Vec<Record>) -> Result<(), Error> {
        let cancel = AtomicBool::new(false);
        let document = Document::new(next);
        let bytes =
            crate::session_write::encode_bounded(&document, &cancel).map_err(|_| Error::Limit)?;
        Document::decode(&bytes)?;
        let retained = if let Some(expected) = &state.original {
            let published = crate::session_file_replace::Replacement::prepare_bytes(
                &self.0.path,
                expected,
                bytes.clone(),
                &cancel,
            )
            .map_err(|_| Error::Changed)?
            .commit(&cancel)
            .map_err(|_| Error::Write)?;
            published.retained_recovery.is_some()
        } else {
            crate::session_file_write::Staged::prepare_bytes(&self.0.path, bytes.clone(), &cancel)
                .map_err(|_| Error::Write)?
                .commit_new(&cancel)
                .map_err(|_| Error::Changed)?;
            false
        };
        state.published(bytes, document, retained)
    }
}
impl Ticket {
    pub(crate) fn intent_id(&self) -> &str {
        &self.record.intent_id
    }
    /// 同一持久记录的串行重试快照；不复制活动许可，worker 仍必须 activate 后才能发网。
    pub(crate) fn retry(&self) -> Self {
        Self {
            store: self.store.clone(),
            record: self.record.clone(),
            permit: None,
        }
    }
    pub(crate) fn same_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.store.0, &other.store.0) && self.record == other.record
    }
    pub fn envelope(&self) -> &Envelope {
        &self.record.envelope
    }
    pub fn origin(&self) -> &str {
        self.record
            .offline
            .as_ref()
            .map_or(self.record.envelope.origin(), |route| &route.origin)
    }
    pub(crate) fn path(&self, operation: super::ReceiveOperation) -> &'static str {
        super::receive_route::path(self.record.offline.is_some(), operation)
    }
    pub fn device(&self) -> &str {
        &self.record.device
    }
    pub fn unit(&self) -> &str {
        &self.record.unit
    }
    pub(crate) fn invitation(&self) -> Invitation {
        self.record.invitation()
    }
    /// 仅由已拿到 unlinked:true 的串行网络 owner 调用；失败保留磁盘记录以供幂等重试。
    pub(crate) fn retire(mut self) -> Result<(), Error> {
        self.activate()?;
        let mut state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
        if !state.entries.iter().any(|old| old == &self.record) {
            return Err(Error::Changed);
        }
        let next = state
            .entries
            .iter()
            .filter(|old| *old != &self.record)
            .cloned()
            .collect();
        self.store.persist(&mut state, next)
    }
}
impl std::fmt::Debug for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionReceiveIntent")
            .field("unit", &self.unit())
            .finish_non_exhaustive()
    }
}
fn regular(file: &File) -> Result<(), Error> {
    let meta = file.metadata().map_err(|_| Error::Read)?;
    if !meta.is_file() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::Read);
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
        Err(_) => return Err(Error::Read),
    };
    regular(&file)?;
    let mut budget = crate::pixels::reserve(MAX_BYTES + 1).ok_or(Error::Limit)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_BYTES + 1)
        .map_err(|_| Error::Limit)?;
    if !budget.grow_to(bytes.capacity()) {
        return Err(Error::Limit);
    }
    // 固定容量读到边界/EOF，避免 read_to_end 在满缓冲的 EOF 探测时再次扩容。
    bytes.resize(MAX_BYTES + 1, 0);
    let mut reader = file;
    let mut length = 0;
    while length < bytes.len() {
        let count = reader.read(&mut bytes[length..]).map_err(|_| Error::Read)?;
        if count == 0 {
            break;
        }
        length += count;
    }
    if length > MAX_BYTES {
        return Err(Error::Limit);
    }
    bytes.truncate(length);
    Ok(Some(Pixels::from_reserved(bytes, budget)))
}
#[cfg(test)]
#[path = "projection_receive_journal_tests.rs"]
pub(crate) mod tests;
