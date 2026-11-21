//! 显式文本复制的有界 UTF-16 暂存；复用剪贴板内存预算和所有权转移。
use super::*;
fn units(text: &str) -> Result<Vec<u16>> {
    bounded_units(text, 65536, 256 * 1024, false)
}
fn bounded_units(text: &str, limit: usize, bytes: usize, empty: bool) -> Result<Vec<u16>> {
    if (!empty && text.is_empty())
        || text.len() > bytes
        || text.contains('\0')
        || text.encode_utf16().count() > limit
    {
        return Err(Error::new(E_INVALIDARG, "Invalid clipboard selection text"));
    }
    let mut output = Vec::with_capacity(text.len().min(limit) + 1);
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' || ch == '\n' {
            if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            output.extend([13, 10]);
        } else {
            let mut buffer = [0; 2];
            output.extend_from_slice(ch.encode_utf16(&mut buffer));
        }
    }
    output.push(0);
    Ok(output)
}
unsafe fn prepare_text(text: &str) -> Result<Prepared> {
    prepare_units(units(text)?)
}
unsafe fn prepare_units(units: Vec<u16>) -> Result<Prepared> {
    prepare(units.len() * 2, |bytes| {
        for (destination, unit) in bytes.chunks_exact_mut(2).zip(units) {
            destination.copy_from_slice(&unit.to_le_bytes());
        }
        Ok(())
    })
}
pub(in crate::desktop) unsafe fn copy_checked(
    owner: HWND,
    text: &str,
    current: impl FnOnce() -> bool,
) -> Result<()> {
    let prepared = prepare_text(text)?;
    publish(owner, prepared, current)
}
pub(in crate::desktop) unsafe fn copy_full_checked(
    owner: HWND,
    text: &str,
    current: impl FnOnce() -> bool,
) -> Result<()> {
    // 全文入口不沿扩展命令的截取策略；超限/无效文本在触碰原剪贴板前明确拒绝。
    let prepared = prepare_units(full_units(text)?)?;
    publish(owner, prepared, current)
}
fn full_units(text: &str) -> Result<Vec<u16>> {
    bounded_units(text, 20 * 1024 * 1024, 20 * 1024 * 1024, false)
}
pub(in crate::desktop) unsafe fn copy_extension_checked(
    owner: HWND,
    text: &str,
    current: impl FnOnce() -> bool,
) -> Result<()> {
    // 主干扩展复制截取至 1 Mi UTF-16 字符；保留完整 Unicode scalar，不切开代理对。
    let mut count = 0;
    let end = text
        .char_indices()
        .find_map(|(i, c)| {
            count += c.len_utf16();
            (count > 1_048_576).then_some(i)
        })
        .unwrap_or(text.len());
    let prepared = prepare_units(bounded_units(
        &text[..end],
        1_048_576,
        4 * 1024 * 1024,
        true,
    )?)?;
    publish(owner, prepared, current)
}
unsafe fn publish(owner: HWND, prepared: Prepared, current: impl FnOnce() -> bool) -> Result<()> {
    OpenClipboard(Some(owner))?;
    let _clipboard = Clipboard;
    // 持锁后、清空前复核窗口/选区；失败不得擦除用户原剪贴板。
    if !current() {
        return Err(Error::new(
            E_ABORT,
            "Surface selection changed before clipboard publication",
        ));
    }
    EmptyClipboard()?;
    prepared.transfer(13) // CF_UNICODETEXT；系统发布失败无法恢复原第三方格式。
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_selection_text_is_crlf_utf16_and_null_terminated_without_clipboard_access() {
        let text = "中文😀\nsecond\r\nthird\rlast";
        let expected: Vec<_> = "中文😀\r\nsecond\r\nthird\r\nlast\0"
            .encode_utf16()
            .collect();
        assert_eq!(units(text).unwrap(), expected);
        unsafe {
            let prepared = prepare_text(text).unwrap();
            let memory = prepared.memory.as_ref().unwrap().0;
            let pointer = GlobalLock(memory);
            assert!(!pointer.is_null());
            let lock = Locked(memory);
            assert_eq!(
                std::slice::from_raw_parts(pointer.cast::<u16>(), expected.len()),
                expected
            );
            drop(lock);
            drop(prepared);
        }
    }
    #[test]
    fn clipboard_selection_text_bounds_reject_nul_empty_and_oversized_inputs() {
        for text in [
            String::new(),
            "a\0b".into(),
            "a".repeat(65537),
            "😀".repeat(32769),
        ] {
            assert!(units(&text).is_err());
        }
        assert_eq!(units(&"\n".repeat(65536)).unwrap().len(), 131073);
    }
    #[test]
    fn extension_text_has_its_own_main_budget_without_relaxing_surface_selection() {
        assert!(units(&"a".repeat(65537)).is_err());
        assert_eq!(
            bounded_units(&"a".repeat(70_000), 1_048_576, 4 * 1024 * 1024, true)
                .unwrap()
                .len(),
            70_001
        );
        assert_eq!(
            bounded_units("", 1_048_576, 4 * 1024 * 1024, true).unwrap(),
            vec![0]
        );
        assert!(bounded_units("a\0b", 1_048_576, 4 * 1024 * 1024, true).is_err());
    }
    #[test]
    fn cached_full_text_is_not_truncated_at_selection_or_extension_command_limits() {
        let text = format!("{}😀\n末尾", "a".repeat(1_048_577));
        let units = full_units(&text).unwrap();
        assert_eq!(
            String::from_utf16(&units[..units.len() - 1]).unwrap(),
            text.replace('\n', "\r\n")
        );
        assert!(full_units("").is_err());
        assert!(full_units("a\0b").is_err());
        assert!(full_units(&"x".repeat(20 * 1024 * 1024 + 1)).is_err());
    }
}
