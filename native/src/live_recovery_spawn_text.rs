//! 私有同 EXE 启动参数：明确 application path、CRT 引号规则和有界 UTF-16 环境块。
use std::{ffi::OsStr, io, os::windows::ffi::OsStrExt, process::Command};
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Invalid recovery process arguments",
    )
}
fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let value: Vec<_> = value.encode_wide().take(32768).collect();
    if value.len() > 32766 || value.contains(&0) {
        return Err(invalid());
    }
    Ok(value)
}
pub(super) fn executable(command: &Command) -> io::Result<Vec<u16>> {
    if command.get_program() != std::env::current_exe()?.as_os_str()
        || command.get_current_dir().is_some()
    {
        return Err(invalid());
    }
    let mut value = wide(command.get_program())?;
    value.push(0);
    Ok(value)
}
pub(super) fn arguments(command: &Command) -> io::Result<Vec<u16>> {
    let mut output = Vec::new();
    for value in std::iter::once(command.get_program()).chain(command.get_args()) {
        if !output.is_empty() {
            output.push(32);
        }
        output.push(34);
        let mut slashes = 0;
        for unit in wide(value)? {
            if unit == 92 {
                slashes += 1;
                continue;
            }
            output.extend(std::iter::repeat_n(
                92,
                if unit == 34 { slashes * 2 + 1 } else { slashes },
            ));
            slashes = 0;
            output.push(unit);
        }
        output.extend(std::iter::repeat_n(92, slashes * 2));
        output.push(34);
        if output.len() > 32766 {
            return Err(invalid());
        }
    }
    output.push(0);
    Ok(output)
}
pub(super) fn environment(command: &Command) -> io::Result<Option<Vec<u16>>> {
    if command.get_envs().next().is_none() {
        return Ok(None);
    }
    // 仅测试启动需要覆盖 ASCII fixture 键；生产继承原环境，不重新编码环境块。
    let mut values: Vec<_> = std::env::vars_os().collect();
    for (key, value) in command.get_envs() {
        let name = key
            .to_str()
            .filter(|name| name.is_ascii() && !name.is_empty() && !name.contains(['=', '\0']))
            .ok_or_else(invalid)?;
        values.retain(|(key, _)| {
            !key.to_str()
                .is_some_and(|key| key.eq_ignore_ascii_case(name))
        });
        if let Some(value) = value {
            values.push((key.to_owned(), value.to_owned()));
        }
    }
    values.sort_by_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (key, value) in values {
        block.extend(wide(&key)?);
        block.push(61);
        block.extend(wide(&value)?);
        block.push(0);
        if block.len() > 1_048_576 {
            return Err(invalid());
        }
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    Ok(Some(block))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting_preserves_empty_unicode_quotes_and_trailing_backslash() {
        let mut command = Command::new("exe");
        command.args(["", "a\"b", "C:\\tail\\", "中文 & x"]);
        let value = arguments(&command).unwrap();
        assert_eq!(value.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&value[..value.len() - 1]).unwrap(),
            "\"exe\" \"\" \"a\\\"b\" \"C:\\tail\\\\\" \"中文 & x\""
        );
    }
    #[test]
    fn invalid_text_and_non_current_executable_are_rejected() {
        assert!(executable(&Command::new("other.exe")).is_err());
        assert!(arguments(Command::new("exe").arg("bad\0value")).is_err());
        assert!(arguments(Command::new("exe").arg("x".repeat(32767))).is_err());
        assert!(
            executable(Command::new(std::env::current_exe().unwrap()).current_dir(".")).is_err()
        );
    }
    #[test]
    fn environment_override_is_local_and_double_null_terminated() {
        let mut command = Command::new("exe");
        assert!(environment(&command).unwrap().is_none());
        command.env("OTTER_RECOVERY_TEXT_FIXTURE", "中文=retained");
        let block = environment(&command).unwrap().unwrap();
        assert!(block.ends_with(&[0, 0]));
        let text = String::from_utf16_lossy(&block);
        assert!(text.contains("OTTER_RECOVERY_TEXT_FIXTURE=中文=retained\0"));
        command.env_remove("OTTER_RECOVERY_TEXT_FIXTURE");
        let block = environment(&command).unwrap().unwrap();
        assert!(!String::from_utf16_lossy(&block).contains("OTTER_RECOVERY_TEXT_FIXTURE="));
    }
}
