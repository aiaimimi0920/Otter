//! 显式配置来源：原始字节用于磁盘冲突检测，文档用于保留未知字段；不自动读写路径。
use crate::{
    app_settings_document::{Document, MAX_BYTES},
    pixels::Pixels,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Source {
    path: PathBuf,
    bytes: Pixels,
    document: Arc<Document>,
    recovery_target: Option<PathBuf>,
}
impl Source {
    pub fn new(path: PathBuf, bytes: Vec<u8>) -> Result<Arc<Self>, String> {
        let document = Document::decode(&bytes)?;
        Self::encoded(path, bytes, document)
    }
    pub fn recovered(
        target: PathBuf,
        recovery: PathBuf,
        bytes: Vec<u8>,
    ) -> Result<Arc<Self>, String> {
        if !target.is_absolute() {
            return Err("Recovery target must be absolute".into());
        }
        let mut source = Self::new(recovery, bytes)?;
        Arc::get_mut(&mut source)
            .expect("new source has one owner")
            .recovery_target = Some(target);
        Ok(source)
    }
    fn encoded(path: PathBuf, bytes: Vec<u8>, document: Document) -> Result<Arc<Self>, String> {
        if !path.is_absolute() || bytes.len() > MAX_BYTES {
            return Err("Invalid settings source path or size".into());
        }
        let lease = crate::pixels::reserve(bytes.capacity())
            .ok_or("Settings source byte budget exhausted")?;
        Ok(Arc::new(Self {
            path,
            bytes: Pixels::from_reserved(bytes, lease),
            document: Arc::new(document),
            recovery_target: None,
        }))
    }
    pub(crate) fn from_document(path: PathBuf, document: Document) -> Result<Arc<Self>, String> {
        if !path.is_absolute() {
            return Err("Settings source path must be absolute".into());
        }
        let mut lease =
            crate::pixels::reserve(MAX_BYTES).ok_or("Settings source byte budget exhausted")?;
        let bytes = document.encode()?;
        if !lease.grow_to(bytes.capacity()) {
            return Err("Settings source byte budget exhausted".into());
        }
        Ok(Arc::new(Self {
            path,
            bytes: Pixels::from_reserved(bytes, lease),
            document: Arc::new(document),
            recovery_target: None,
        }))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn recovery_target(&self) -> Option<&Path> {
        self.recovery_target.as_deref()
    }
    pub fn document(&self) -> &Arc<Document> {
        &self.document
    }
    pub(crate) fn bytes(&self) -> &Pixels {
        &self.bytes
    }
}
