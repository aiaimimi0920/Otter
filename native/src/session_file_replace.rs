//! 有恢复路径的分阶段替换；保留竞争写者，不宣称两个 rename 合成原子覆盖。
use crate::{session_file_guard::Movable, session_file_write::Staged, session_write::Candidate};
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

#[derive(Debug)]
pub struct Failure {
    pub error: io::Error,
    pub recovery: Option<PathBuf>,
    pub restore_error: Option<io::Error>,
}
impl Failure {
    fn before(error: io::Error) -> Self {
        Self {
            error,
            recovery: None,
            restore_error: None,
        }
    }
}
#[derive(Debug)]
pub struct Published {
    pub destination: PathBuf,
    /// 新内容已成功发布，但旧恢复文件退休失败；调用者须显示，不得伪报整个保存失败。
    pub retained_recovery: Option<(PathBuf, io::Error)>,
}
pub struct Replacement {
    staged: Option<Staged>,
    source: Movable,
    destination: PathBuf,
    recovery: PathBuf,
    restore_on_drop: bool,
    // 最后释放目录句柄；暂存清理、旧文件恢复期间不能先解除目录重命名限制。
    _directory: File,
}
impl Replacement {
    pub fn prepare(
        destination: &Path,
        expected: &[u8],
        candidate: &Candidate,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        check(cancel)?;
        if !destination.is_absolute() {
            return Err(invalid("Replacement path must be absolute"));
        }
        let original = crate::session_document::decode(expected)
            .map_err(|_| invalid("Invalid replacement baseline"))?;
        if original.revision != candidate.expected_revision {
            return Err(invalid("Replacement revision does not match baseline"));
        }
        drop(original);
        Self::prepare_with(destination, expected, cancel, |path| {
            Staged::prepare(path, candidate, cancel)
        })
    }
    /// 格式 owner 已验证的字节；保留完全相同的基线锁定、恢复及竞争处理。
    pub(crate) fn prepare_bytes(
        destination: &Path,
        expected: &[u8],
        bytes: crate::pixels::Pixels,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        Self::prepare_with(destination, expected, cancel, |path| {
            Staged::prepare_bytes(path, bytes, cancel)
        })
    }
    fn prepare_with(
        destination: &Path,
        expected: &[u8],
        cancel: &AtomicBool,
        stage: impl FnOnce(&Path) -> io::Result<Staged>,
    ) -> io::Result<Self> {
        check(cancel)?;
        if !destination.is_absolute() {
            return Err(invalid("Replacement path must be absolute"));
        }
        let name = destination
            .file_name()
            .ok_or_else(|| invalid("Missing replacement filename"))?;
        let (parent, directory) =
            crate::session_file_trace::run("replacement-directory", None, || {
                let parent = destination
                    .parent()
                    .ok_or_else(|| invalid("Missing replacement directory"))?
                    .canonicalize()?;
                let directory = OpenOptions::new()
                    .read(true)
                    .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
                    .open(&parent)?;
                let metadata = directory.metadata()?;
                if !metadata.is_dir()
                    || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
                {
                    return Err(invalid("Invalid replacement directory"));
                }
                Ok::<_, io::Error>((parent, directory))
            })?;
        let destination = parent.join(name);
        let source = Movable::open(&destination, expected, cancel)?;
        let mut recovery_name = name.to_os_string();
        let id =
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.map_err(io::Error::other)?;
        recovery_name.push(format!(".otter-recovery-{id:?}.json"));
        let recovery = parent.join(recovery_name);
        let staged = stage(&destination)?;
        Ok(Self {
            staged: Some(staged),
            source,
            destination,
            recovery,
            restore_on_drop: false,
            _directory: directory,
        })
    }
    pub fn is_for(&self, candidate: &Candidate) -> bool {
        self.staged
            .as_ref()
            .is_some_and(|staged| staged.is_for(candidate))
    }
    /// 调用者仍须先核对 UI Receipt。旧文件移开到新文件发布存在可见间隔；
    /// 任何竞争目标都保留，无法恢复时报告旧文件路径，不悄悄覆盖或删除对方文件。
    pub fn commit(self, cancel: &AtomicBool) -> Result<Published, Failure> {
        self.commit_with(cancel, |_| {})
    }
    fn commit_with(
        self,
        cancel: &AtomicBool,
        before_publish: impl FnOnce(&Path),
    ) -> Result<Published, Failure> {
        self.commit_observed(cancel, before_publish, |_| {})
    }
    // 私有同步观察点供进程中断回归使用；正式调用始终传入空回调，不暴露故障注入入口。
    fn commit_observed(
        mut self,
        cancel: &AtomicBool,
        before_publish: impl FnOnce(&Path),
        after_publish: impl FnOnce(&Path),
    ) -> Result<Published, Failure> {
        check(cancel).map_err(Failure::before)?;
        crate::session_file_trace::run("baseline-move", None, || {
            self.source.rename_new(&self.recovery)
        })
        .map_err(Failure::before)?;
        self.restore_on_drop = true;
        before_publish(&self.destination);
        match self.staged.take().unwrap().commit_new(cancel) {
            Ok(destination) => {
                self.restore_on_drop = false;
                after_publish(&destination);
                let retained_recovery =
                    crate::session_file_trace::run("baseline-retire", None, || {
                        self.source.retire()
                    })
                    .err()
                    .map(|error| (self.recovery.clone(), error));
                Ok(Published {
                    destination,
                    retained_recovery,
                })
            }
            Err(error) => {
                let restore_error = self.source.rename_new(&self.destination).err();
                self.restore_on_drop = false;
                let recovery = restore_error.as_ref().map(|_| self.recovery.clone());
                Err(Failure {
                    error,
                    recovery,
                    restore_error,
                })
            }
        }
    }
}
impl Drop for Replacement {
    fn drop(&mut self) {
        if self.restore_on_drop {
            // panic 路径只做不覆盖恢复；失败保留可识别恢复文件，不删除唯一旧内容。
            let _ = self.source.rename_new(&self.destination);
        }
    }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Session replacement cancelled",
        ))
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "session_file_crash_tests.rs"]
mod crash_tests;
#[cfg(test)]
#[path = "session_file_replace_tests.rs"]
mod tests;
