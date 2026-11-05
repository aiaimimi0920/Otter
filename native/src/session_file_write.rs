//! 同目录分阶段另存为；只提交新文件，不提供覆盖已有会话的伪 CAS 接口。
use crate::{session_document::decode, session_write::Candidate};
use std::os::windows::ffi::OsStrExt;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH},
        System::Com::CoCreateGuid,
    },
};

pub struct Staged {
    file: Option<File>,
    temporary: PathBuf,
    destination: PathBuf,
    committed: bool,
    bytes: Option<crate::pixels::Pixels>,
    revisions: Option<(u64, u64)>,
}
impl Staged {
    /// 调用者显式选择目标；只写本次唯一临时文件，取消/失败不触碰已有目标。
    pub fn prepare(
        destination: &Path,
        candidate: &Candidate,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        check(cancel)?;
        let document =
            decode(&candidate.bytes).map_err(|_| invalid("Invalid session candidate"))?;
        if document.revision != candidate.revision
            || candidate.expected_revision.checked_add(1) != Some(candidate.revision)
        {
            return Err(invalid("Invalid session candidate revision"));
        }
        let mut staged = Self::prepare_bytes(destination, candidate.bytes.clone(), cancel)?;
        staged.revisions = Some((candidate.expected_revision, candidate.revision));
        Ok(staged)
    }
    /// 已由所属格式验证的不可变编码结果；不绑定会话候选身份。
    pub(crate) fn prepare_bytes(
        destination: &Path,
        bytes: crate::pixels::Pixels,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        let size = Some(bytes.len());
        let mut staged = crate::session_file_trace::run("staging-open", size, || {
            Self::prepare_file(destination, cancel)
        })?;
        crate::session_file_trace::run("write", size, || {
            for chunk in bytes.chunks(64 * 1024) {
                check(cancel)?;
                staged.file.as_mut().unwrap().write_all(chunk)?;
            }
            Ok::<_, io::Error>(())
        })?;
        crate::session_file_trace::run("sync", size, || staged.file.as_ref().unwrap().sync_all())?;
        check(cancel)?;
        staged.bytes = Some(bytes);
        Ok(staged)
    }
    /// 格式 owner 流式写入本次独占文件；不保留完整编码缓冲，也不伪造会话候选身份。
    pub(crate) fn prepare_stream(
        destination: &Path,
        cancel: &AtomicBool,
        write: impl FnOnce(&mut File, &AtomicBool) -> io::Result<()>,
    ) -> io::Result<Self> {
        let mut staged = crate::session_file_trace::run("staging-open", None, || {
            Self::prepare_file(destination, cancel)
        })?;
        crate::session_file_trace::run("stream-write", None, || {
            write(staged.file.as_mut().unwrap(), cancel)
        })?;
        crate::session_file_trace::run("sync", None, || staged.file.as_ref().unwrap().sync_all())?;
        check(cancel)?;
        Ok(staged)
    }
    fn prepare_file(destination: &Path, cancel: &AtomicBool) -> io::Result<Self> {
        check(cancel)?;
        if !destination.is_absolute() {
            return Err(invalid("Save destination must be absolute"));
        }
        let name = destination
            .file_name()
            .ok_or_else(|| invalid("Missing save filename"))?;
        if name.encode_wide().any(|c| c == 0 || c == b':' as u16) {
            return Err(invalid("Invalid save filename"));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("Missing save directory"))?
            .canonicalize()?;
        if !parent.is_dir() {
            return Err(invalid("Save parent is not a directory"));
        }
        let destination = parent.join(name);
        let guid = unsafe { CoCreateGuid() }.map_err(io::Error::other)?;
        let temporary = parent.join(format!(".otter-save-{guid:?}.tmp"));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let staged = Self {
            file: Some(file),
            temporary,
            destination,
            committed: false,
            bytes: None,
            revisions: None,
        };
        Ok(staged)
    }
    /// 绑定同一不可变编码结果，不以碰巧相同的修订号或字节长度冒充同一候选。
    pub fn is_for(&self, candidate: &Candidate) -> bool {
        self.bytes
            .as_ref()
            .is_some_and(|bytes| bytes.same_storage(&candidate.bytes))
            && self.revisions == Some((candidate.expected_revision, candidate.revision))
    }
    /// 提交前调用者仍须复核 UI Receipt；不覆盖，不跨卷退化为 copy/delete。
    /// 成功后即使收到迟到取消也不删除已发布文件；断电持久性取决于实际文件系统。
    pub fn commit_new(mut self, cancel: &AtomicBool) -> io::Result<PathBuf> {
        check(cancel)?;
        self.file.take();
        let source: Vec<u16> = self
            .temporary
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let target: Vec<u16> = self
            .destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        check(cancel)?;
        crate::session_file_trace::run("publish-new", None, || unsafe {
            MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(target.as_ptr()),
                MOVEFILE_WRITE_THROUGH,
            )
        })
        .map_err(io::Error::other)?;
        self.committed = true;
        Ok(self.destination.clone())
    }

    /// 已编码资源的递增发布；复用同一个暂存文件，同名冲突不重写内容。
    /// 仅供无会话修订绑定的资源使用；会话仍必须发布到用户确认的确切路径。
    pub(crate) fn commit_incremented(
        mut self,
        stem: &str,
        extension: Option<&str>,
        cancel: &AtomicBool,
    ) -> io::Result<PathBuf> {
        check(cancel)?;
        if self.revisions.is_some() {
            return Err(invalid("Session candidates require an exact destination"));
        }
        self.file.take();
        let source: Vec<u16> = self
            .temporary
            .as_os_str()
            .encode_wide()
            .chain([0])
            .collect();
        let parent = self
            .destination
            .parent()
            .ok_or_else(|| invalid("Missing resource directory"))?;
        let (_, destination) =
            crate::file_naming::allocate::allocate_with(parent, stem, extension, cancel, |path| {
                let target: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
                check(cancel)?;
                // 不设置 REPLACE_EXISTING 或 COPY_ALLOWED，始终同卷无覆盖重命名。
                unsafe {
                    MoveFileExW(
                        PCWSTR(source.as_ptr()),
                        PCWSTR(target.as_ptr()),
                        MOVEFILE_WRITE_THROUGH,
                    )
                }
                .map_err(|error| io::Error::from_raw_os_error(error.code().0 & 0xffff))
            })?;
        self.committed = true;
        Ok(destination)
    }
}
impl Drop for Staged {
    fn drop(&mut self) {
        self.file.take();
        if !self.committed {
            // 只清理本对象 create_new 成功创建的路径；不删除目标或扫描整个目录。
            let _ = std::fs::remove_file(&self.temporary);
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
            "Session write cancelled",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_file_write_tests.rs"]
mod tests;
