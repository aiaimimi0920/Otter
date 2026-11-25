//! Verify origins, sharing locks, and preallocation budgets only with self-created linshi files; never modify system fonts.
use super::*;
fn directory() -> PathBuf {
    let path = crate::test_temp_root().join(format!("otter-font-file-{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir(&path).unwrap();
    path
}
#[test]
fn svg_font_file_same_handle_budget_and_write_exclusion() {
    let path = directory();
    let font = path.join("fixture.ttf");
    std::fs::write(&font, b"font fixture").unwrap();
    let root = Root::open(&path).unwrap();
    assert_eq!(root.read(&font, 12).unwrap(), b"font fixture");
    assert!(root.read(&font, 11).is_err());
    let file = root.admit(&font).unwrap();
    assert!(OpenOptions::new().write(true).open(&font).is_err());
    assert!(std::fs::rename(&font, path.join("renamed.ttf")).is_err());
    drop(file);
    let large = path.join("large.ttf");
    File::create(&large)
        .unwrap()
        .set_len(MAX_FILE as u64 + 1)
        .unwrap();
    assert!(root.read(&large, usize::MAX).is_err());
    let empty = path.join("empty.ttf");
    std::fs::write(&empty, []).unwrap();
    assert!(root.read(&empty, usize::MAX).is_err());
    drop(root);
    for name in [&font, &large, &empty] {
        std::fs::remove_file(name).unwrap();
    }
    std::fs::remove_dir(path).unwrap();
}
#[test]
fn svg_font_file_rejects_other_roots_streams_devices_and_junction_root() {
    let path = directory();
    let sibling = directory();
    let font = sibling.join("outside.ttf");
    std::fs::write(&font, b"outside").unwrap();
    let root = Root::open(&path).unwrap();
    assert!(root.read(&font, 100).is_err());
    for bad in [
        "C:\\x.ttf:stream",
        "\\\\server\\fonts\\x.ttf",
        "\\\\?\\C:\\x.ttf",
        "C:x.ttf",
        "NUL",
        "C:\\x\0.ttf",
    ] {
        assert!(root.read(Path::new(bad), 100).is_err(), "{bad:?}");
    }
    for name in ["CON.ttf", "NUL.otf", "COM1.ttc", "LPT¹.ttf", "CON .ttf"] {
        assert!(!safe_name(&path.join(name)));
    }
    // Junctions require no file-symlink privilege; create links only within the current test directory.
    let link = path.join("junction");
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:OTTER_FONT_TEST_LINK -Target $env:OTTER_FONT_TEST_TARGET | Out-Null"])
        .env("OTTER_FONT_TEST_LINK", &link).env("OTTER_FONT_TEST_TARGET", &sibling).status().unwrap();
    assert!(status.success());
    assert!(Root::open(&link).is_err());
    assert!(root.read(&link.join("outside.ttf"), 100).is_err());
    // remove_dir removes only the link itself without recursively handling its target.
    std::fs::remove_dir(&link).unwrap();
    drop(root);
    std::fs::remove_file(font).unwrap();
    std::fs::remove_dir(path).unwrap();
    std::fs::remove_dir(sibling).unwrap();
}
