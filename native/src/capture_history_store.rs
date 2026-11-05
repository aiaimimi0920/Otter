//! 独立profile历史存储；坏源停止写，完整字节基线/原有恢复事务保护竞争写者。
use super::{Data, MAX_BYTES};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
};

pub(crate) struct Store {
    path: PathBuf,
    expected: Option<crate::pixels::Pixels>,
    _lease: File,
    _directories: Vec<File>,
}
impl Store {
    /// 只读取调用者选定的原生profile，不自动探测或迁移旧Tauri目录。
    pub fn open(root: &Path) -> Result<(Self, Data), String> {
        let directories = crate::profile_directory::pin(root).map_err(|e| e.to_string())?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(root.join("history.lock"))
            .map_err(|e| e.to_string())?;
        regular(&lease).map_err(|e| e.to_string())?;
        let path = root.join("history.json");
        let expected = read(&path).map_err(|e| e.to_string())?;
        let data = match &expected {
            Some(bytes) => Data::decode(bytes)?,
            None => Data::default(),
        };
        Ok((
            Self {
                path,
                expected,
                _lease: lease,
                _directories: directories,
            },
            data,
        ))
    }
    pub fn save(&mut self, data: &Data) -> Result<(), String> {
        // 根扩展最多来自8MiB输入，加上两份24条候选，编码峰值仍有界。
        let bytes = serde_json::to_vec(data).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("History output exceeds 8 MiB; original preserved".into());
        }
        let lease =
            crate::pixels::reserve(bytes.capacity()).ok_or("History storage budget exhausted")?;
        let bytes = crate::pixels::Pixels::from_reserved(bytes, lease);
        let cancel = AtomicBool::new(false);
        if let Some(expected) = &self.expected {
            let published = crate::session_file_replace::Replacement::prepare_bytes(
                &self.path,
                expected,
                bytes.clone(),
                &cancel,
            )
            .map_err(|e| e.to_string())?
            .commit(&cancel)
            .map_err(|e| format!("History publication failed: {e:?}"))?;
            self.expected = Some(bytes);
            if published.retained_recovery.is_some() {
                // 新候选已成功发布，旧恢复文件保留；不重放写入。
                return Err(
                    "History saved with retained recovery file; further writes stopped".into(),
                );
            }
        } else {
            crate::session_file_write::Staged::prepare_bytes(&self.path, bytes.clone(), &cancel)
                .map_err(|e| e.to_string())?
                .commit_new(&cancel)
                .map_err(|e| e.to_string())?;
            self.expected = Some(bytes);
        }
        Ok(())
    }
}
fn regular(file: &File) -> io::Result<u64> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "History requires regular non-reparse files",
        ));
    }
    Ok(metadata.len())
}
fn read(path: &Path) -> io::Result<Option<crate::pixels::Pixels>> {
    let mut file = match OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let size = regular(&file)?;
    if size > MAX_BYTES as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "History exceeds 8 MiB",
        ));
    }
    let lease = crate::pixels::reserve(size as usize)
        .ok_or_else(|| io::Error::other("History read budget exhausted"))?;
    let mut bytes = vec![0; size as usize];
    file.read_exact(&mut bytes)?;
    Ok(Some(crate::pixels::Pixels::from_reserved(bytes, lease)))
}
