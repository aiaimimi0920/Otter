//! 按显式目标发现同目录恢复副本；只读、有界、歧义拒绝，不恢复路径或删除任何文件。
use std::{
    io,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
const MAX_ENTRIES: usize = 4096;
pub fn find(destination: &Path, cancel: &AtomicBool) -> io::Result<Option<PathBuf>> {
    check(cancel)?;
    if !destination.is_absolute() {
        return Err(invalid("Recovery target must be absolute"));
    }
    if destination.try_exists()? {
        return Ok(None);
    }
    let name = destination
        .file_name()
        .ok_or_else(|| invalid("Missing recovery target name"))?;
    let mut prefix: Vec<u16> = name.encode_wide().take(32768).collect();
    if prefix.len() > 32767 || prefix.iter().any(|c| *c == 0 || *c == b':' as u16) {
        return Err(invalid("Invalid recovery target name"));
    }
    prefix.extend(".otter-recovery-".encode_utf16());
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("Missing recovery directory"))?;
    let mut found = None;
    for (index, entry) in std::fs::read_dir(parent)?.enumerate() {
        check(cancel)?;
        if index >= MAX_ENTRIES {
            return Err(invalid("Recovery directory exceeds scan budget"));
        }
        let entry = entry?;
        let name: Vec<u16> = entry.file_name().encode_wide().take(32768).collect();
        if !matches_name(&name, &prefix) || !entry.file_type()?.is_file() {
            continue;
        }
        if found.is_some() {
            return Err(invalid(
                "Multiple session recovery copies; select one explicitly",
            ));
        }
        found = Some(entry.path());
    }
    check(cancel)?;
    Ok(found)
}
fn matches_name(name: &[u16], prefix: &[u16]) -> bool {
    if name.len() != prefix.len() + 36 + 5
        || !name.starts_with(prefix)
        || !name.ends_with(&[
            b'.' as u16,
            b'j' as u16,
            b's' as u16,
            b'o' as u16,
            b'n' as u16,
        ])
    {
        return false;
    }
    name[prefix.len()..prefix.len() + 36]
        .iter()
        .enumerate()
        .all(|(index, c)| {
            if [8, 13, 18, 23].contains(&index) {
                *c == b'-' as u16
            } else {
                u8::try_from(*c).is_ok_and(|c| c.is_ascii_hexdigit())
            }
        })
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Recovery search cancelled",
        ))
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "session_file_recovery_tests.rs"]
mod tests;
