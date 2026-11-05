//! 显式文本复制的有界 UTF-16 暂存；复用剪贴板内存预算和所有权转移。
use super::*;
fn units(text: &str) -> Result<Vec<u16>> {
    if text.is_empty()
        || text.len() > 256 * 1024
        || text.contains('\0')
        || text.encode_utf16().count() > 65536
    {
        return Err(Error::new(E_INVALIDARG, "Invalid clipboard selection text"));
    }
    let mut output = Vec::with_capacity(text.len().min(65536) + 1);
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
    let units = units(text)?;
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
}
