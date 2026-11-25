//! Owns one disposable host Art cache/profile fixture, never a historical run or real profile.
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

pub(crate) struct Directory {
    base: PathBuf,
    root: PathBuf,
}

impl Directory {
    pub(crate) fn new() -> Self {
        let temporary = Path::new(r"\\?\C:\Users\Public\nas_home\AI\GameEditor\linshi");
        assert_eq!(temporary.canonicalize().unwrap(), temporary);
        let base = temporary.join("otter-host-art-cache-tests");
        fs::create_dir_all(&base).unwrap();
        assert_eq!(base.canonicalize().unwrap(), base);
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let root = base.join(format!("fixture-{id:?}"));
        fs::create_dir(&root).unwrap();
        Self { base, root }
    }
}

impl Deref for Directory {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.root
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let result = self.root.canonicalize().and_then(|path| {
            if path != self.root || path.parent() != Some(self.base.as_path()) {
                return Err(std::io::Error::other("Art cache fixture path changed"));
            }
            fs::remove_dir_all(path)
        });
        if !std::thread::panicking() {
            result.expect("remove owned Art cache fixture");
        }
    }
}
