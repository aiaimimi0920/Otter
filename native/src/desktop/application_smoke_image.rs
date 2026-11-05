//! 原生启动探针的内嵌图片：文件链仍走真实磁盘输入，但不依赖构建仓库。
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::E_FAIL, System::Com::CoCreateGuid},
};

pub(in crate::desktop::application) const PNG: &[u8] =
    include_bytes!("../../../src-tauri/icons/32x32.png");

pub(in crate::desktop::application) struct ImageFile(PathBuf);

impl ImageFile {
    pub(in crate::desktop::application) fn new() -> Result<Self> {
        Self::in_directory(&std::env::temp_dir())
    }

    fn in_directory(directory: &Path) -> Result<Self> {
        let id = unsafe { CoCreateGuid() }?;
        let path = directory.join(format!("otter-native-smoke-{id:?}.png"));
        // create_new 成功后才取得清理责任；不覆盖或删除已有文件。
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        let owned = Self(path);
        let written = file.write_all(PNG);
        drop(file); // WIC 以独立句柄读取；先关闭写句柄，包括失败路径。
        written.map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        Ok(owned)
    }

    pub(in crate::desktop::application) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ImageFile {
    fn drop(&mut self) {
        // 仅删除本次独占创建的单个文件，不遍历临时目录。
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_image_files_are_independent_and_removed_after_use() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let directory = Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        let first = ImageFile::in_directory(directory).unwrap();
        let second = ImageFile::in_directory(directory).unwrap();
        assert_ne!(first.path(), second.path());
        assert_eq!(std::fs::read(first.path()).unwrap(), PNG);
        let image = crate::desktop::image::Image::open(first.path()).unwrap();
        assert_eq!((image.width, image.height), (32, 32));
        drop(image);
        let path = first.path().to_owned();
        drop(first);
        assert!(!path.exists());
        assert!(second.path().exists());
    }
}
