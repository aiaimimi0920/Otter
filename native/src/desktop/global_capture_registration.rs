//! Ctrl+1 拥有唯一宿主；附属 Live/长截图独立降级，关闭失败时恢复已释放的附属键。
use std::cell::Cell;
use windows::core::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::desktop) enum Key {
    Primary = 0,
    Live = 1,
    Long = 2,
}
impl Key {
    fn label(self) -> &'static str {
        match self {
            Self::Primary => "Ctrl+1",
            Self::Live => "Ctrl+2",
            Self::Long => "Ctrl+3",
        }
    }
}
fn problem(key: Key, action: &str, error: Error) -> Error {
    Error::new(
        error.code(),
        format!("{} {action} failed: {error}", key.label()),
    )
}
fn combined(errors: Vec<Error>) -> Error {
    Error::new(
        errors[0].code(),
        errors
            .iter()
            .map(|error| error.message().to_string())
            .collect::<Vec<_>>()
            .join("; "),
    )
}
pub(super) fn cleanup_with(
    flags: [&Cell<bool>; 3],
    mut remove: impl FnMut(Key) -> Result<()>,
) -> [Option<Error>; 3] {
    [Key::Long, Key::Live, Key::Primary].map(|key| {
        let flag = flags[key as usize];
        if !flag.get() {
            return None;
        }
        match remove(key) {
            Ok(()) => {
                flag.set(false);
                None
            }
            Err(error) => Some(problem(key, "cleanup", error)),
        }
    })
}

pub(super) fn set_with(
    flags: [&Cell<bool>; 3],
    enabled: bool,
    mut change: impl FnMut(Key, bool) -> Result<()>,
) -> Result<Option<Error>> {
    let primary = flags[Key::Primary as usize];
    if enabled {
        if !primary.get() {
            change(Key::Primary, true)?;
            primary.set(true);
        }
        let mut warnings = Vec::new();
        for key in [Key::Live, Key::Long] {
            let flag = flags[key as usize];
            if flag.get() {
                continue;
            }
            match change(key, true) {
                Ok(()) => flag.set(true),
                // 一个附属键冲突不能跳过另一个，也不能撤销 Ctrl+1。
                Err(error) => warnings.push(problem(key, "registration", error)),
            }
        }
        return Ok((!warnings.is_empty()).then(|| combined(warnings)));
    }
    let mut removed = [false; 3];
    for key in [Key::Live, Key::Long, Key::Primary] {
        let flag = flags[key as usize];
        if !flag.get() {
            continue;
        }
        if let Err(error) = change(key, false) {
            let mut errors = vec![problem(key, "unregister", error)];
            // 即使一次补偿失败，也继续恢复其他已释放键；始终保留可发现的主 owner。
            for recovery in [Key::Live, Key::Long] {
                if removed[recovery as usize] {
                    match change(recovery, true) {
                        Ok(()) => flags[recovery as usize].set(true),
                        Err(error) => errors.push(problem(recovery, "recovery", error)),
                    }
                }
            }
            return Err(combined(errors));
        }
        flag.set(false);
        removed[key as usize] = true;
    }
    Ok(None)
}

#[cfg(test)]
#[path = "global_capture_registration_tests.rs"]
mod tests;
