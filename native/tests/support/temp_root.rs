//! Shared scratch directory for the host and integration fixtures.
use std::{fs, io, os::windows::fs::MetadataExt, path::PathBuf};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

pub(crate) fn test_temp_root() -> PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .join(".tmp");
    // Create only the leaf; never follow an existing .tmp junction into another tree.
    match fs::create_dir(&root) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => {
            panic!("cannot create repository .tmp: {error}")
        }
        _ => {}
    }
    let entry = fs::symlink_metadata(&root).expect("inspect repository .tmp");
    assert!(
        entry.is_dir() && entry.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0,
        "repository .tmp must be an ordinary directory"
    );
    root
}

/// Check the resolved destination, then return an ordinary path accepted by the profile CLI.
pub(crate) fn fixture_directory(path: &std::path::Path) -> PathBuf {
    assert!(path.is_absolute());
    let base = test_temp_root();
    let directory = path.canonicalize().expect("resolve fixture directory");
    let relative = directory
        .strip_prefix(base.canonicalize().expect("resolve repository .tmp"))
        .expect("fixture must be under repository .tmp");
    assert!(!relative.as_os_str().is_empty() && directory.is_dir());
    base.join(relative)
}

#[test]
fn fixture_directory_only_accepts_children_and_preserves_ordinary_profile_paths() {
    let base = test_temp_root();
    let child = base.join(format!("otter-scratch-{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    fs::create_dir(&child).unwrap();
    assert_eq!(fixture_directory(&child.canonicalize().unwrap()), child);
    assert!(std::panic::catch_unwind(|| fixture_directory(&base)).is_err());
    assert!(std::panic::catch_unwind(|| fixture_directory(base.parent().unwrap())).is_err());
    fs::remove_dir(child).unwrap();
}
