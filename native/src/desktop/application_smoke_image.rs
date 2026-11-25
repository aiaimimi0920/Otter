//! Embedded image for native-startup probes: file flows still use real disk input without depending on the build repository.
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
    include_bytes!("../../assets/branding/32x32.png");

pub(in crate::desktop::application) struct ImageFile(PathBuf);

impl ImageFile {
    pub(in crate::desktop::application) fn new() -> Result<Self> {
        Self::in_directory(&std::env::temp_dir())
    }

    fn in_directory(directory: &Path) -> Result<Self> {
        let id = unsafe { CoCreateGuid() }?;
        let path = directory.join(format!("otter-native-smoke-{id:?}.png"));
        // Acquire cleanup responsibility only after create_new succeeds; never overwrite or delete existing files.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        let owned = Self(path);
        let written = file.write_all(PNG);
        drop(file); // WIC reads through an independent handle; close the write handle first, including failure paths.
        written.map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        Ok(owned)
    }

    pub(in crate::desktop::application) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ImageFile {
    fn drop(&mut self) {
        // Delete only the single file exclusively created by this call, without traversing the temporary directory.
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_image_files_are_independent_and_removed_after_use() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let first = ImageFile::new().unwrap();
        let second = ImageFile::new().unwrap();
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
