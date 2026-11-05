//! 缓存所有权记录：只记录本次已编码内容与已发布文件身份，为后续保守清理提供凭证。
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    },
    System::Com::CoCreateGuid,
};
pub(super) const MAX_BYTES: usize = 2048;
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub version: u32,
    pub name: String,
    pub bytes: u64,
    pub sha256: [u8; 32],
    pub created: u64,
    pub volume: u32,
    pub index: u64,
}
pub(super) struct Draft {
    bytes: u64,
    sha256: [u8; 32],
    created: u64,
}
impl Draft {
    pub fn new(bytes: &[u8]) -> io::Result<Self> {
        Self::from_digest(bytes.len() as u64, Sha256::digest(bytes).into())
    }
    /// 仅接收所属编码器的有界流式写入统计，不接受外部文件声明的长度/哈希。
    pub fn from_digest(bytes: u64, sha256: [u8; 32]) -> io::Result<Self> {
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_secs();
        Ok(Self {
            bytes,
            sha256,
            created,
        })
    }
    pub fn publish(self, path: &Path) -> io::Result<()> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("Invalid cache receipt filename"))?;
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.len() != self.bytes
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        {
            return Err(io::Error::other(
                "Published cache file changed before receipt",
            ));
        }
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
            .map_err(io::Error::other)?;
        if info.nNumberOfLinks != 1 {
            return Err(io::Error::other(
                "Cache receipt requires a single-link file",
            ));
        }
        let record = Receipt {
            version: 1,
            name: name.into(),
            bytes: self.bytes,
            sha256: self.sha256,
            created: self.created,
            volume: info.dwVolumeSerialNumber,
            index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        };
        let lease = crate::pixels::reserve(MAX_BYTES)
            .ok_or_else(|| io::Error::other("Cache receipt memory budget exhausted"))?;
        let mut bytes = Vec::with_capacity(MAX_BYTES);
        serde_json::to_writer(&mut bytes, &record).map_err(io::Error::other)?;
        if bytes.len() > MAX_BYTES {
            return Err(io::Error::other("Cache receipt exceeds limit"));
        }
        let id = unsafe { CoCreateGuid() }.map_err(io::Error::other)?;
        let target = path
            .parent()
            .ok_or_else(|| io::Error::other("Missing cache receipt parent"))?
            .join(format!(".otter-clipboard-{id:?}.json"));
        let cancel = AtomicBool::new(false);
        let bytes = crate::pixels::Pixels::from_reserved(bytes, lease);
        crate::session_file_write::Staged::prepare_bytes(&target, bytes, &cancel)?
            .commit_new(&cancel)?;
        Ok(())
    }
}
