//! Only snapshot UTF-16 while the clipboard is open. Never free system-owned handles.
use crate::text_annotation::MAX_TEXT_UNITS;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::{DataExchange::GetClipboardData, Memory::*},
    },
};
pub(super) const FORMAT: u32 = 13; // CF_UNICODETEXT
fn invalid() -> Error {
    Error::new(E_INVALIDARG, crate::ui_text::text("paste-card-invalid"))
}
pub(super) unsafe fn snapshot() -> Result<Vec<u16>> {
    read_block(HGLOBAL(GetClipboardData(FORMAT)?.0))
}
/// Borrow a live movable memory block; neither success nor error transfers ownership.
unsafe fn read_block(memory: HGLOBAL) -> Result<Vec<u16>> {
    let bytes = GlobalSize(memory);
    if bytes < 2 {
        return Err(invalid());
    }
    let pointer = GlobalLock(memory);
    if pointer.is_null() {
        return Err(Error::from_win32());
    }
    let _lock = super::Lock(memory);
    // GlobalAlloc rounds block sizes to heap granularity; bound the scan to MAX_TEXT_UNITS + 1
    // without falsely rejecting valid max-length or over-allocated clipboard blocks.
    let units =
        std::slice::from_raw_parts(pointer.cast::<u16>(), (bytes / 2).min(MAX_TEXT_UNITS + 1));
    let end = units.iter().position(|&u| u == 0).ok_or_else(invalid)?;
    Ok(units[..end].to_vec())
}
pub(in crate::desktop) fn normalize(units: &[u16]) -> Result<String> {
    if units.len() > MAX_TEXT_UNITS {
        return Err(invalid());
    }
    let raw = String::from_utf16(units).map_err(|_| invalid())?;
    let value = raw.replace("\r\n", "\n").replace('\r', "\n");
    if value.contains('\0')
        || !value.chars().any(|c| !c.is_whitespace() && !c.is_control())
        || value.split('\n').count() > 400
    {
        return Err(invalid());
    }
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(s: &str) -> Result<String> {
        normalize(&s.encode_utf16().collect::<Vec<_>>())
    }
    #[test]
    fn owned_win32_text_block_is_bounded_unlocked_and_preserves_the_first_terminator() {
        unsafe {
            struct Owned(HGLOBAL);
            impl Drop for Owned {
                fn drop(&mut self) {
                    // GlobalFree returns NULL on success, unlike the generated
                    // non-null return wrapper. The final lock count is asserted below.
                    unsafe {
                        let _ = GlobalFree(Some(self.0));
                    }
                }
            }
            let memory = Owned(GlobalAlloc(GMEM_MOVEABLE, 32).unwrap());
            let bytes = GlobalSize(memory.0);
            assert!(bytes >= 32 && bytes % 2 == 0);
            let assign = |head: &[u16]| {
                let ptr = GlobalLock(memory.0).cast::<u16>();
                assert!(!ptr.is_null());
                let lock = super::super::Lock(memory.0);
                let units = std::slice::from_raw_parts_mut(ptr, bytes / 2);
                units.fill(b'x' as u16);
                units[..head.len()].copy_from_slice(head);
                drop(lock);
            };
            assign(&[]);
            assert!(
                read_block(memory.0).is_err(),
                "no NUL anywhere in GlobalSize"
            );
            assign(&[0xd83e, 0xdda6, 0, 0xd800]);
            assert_eq!(normalize(&read_block(memory.0).unwrap()).unwrap(), "🦦");
            assign(&[0xd800, 0]);
            assert!(normalize(&read_block(memory.0).unwrap()).is_err());
            assert_eq!(GlobalFlags(memory.0) & 0xff, 0, "reader leaked a lock");

            let full = Owned(GlobalAlloc(GMEM_MOVEABLE, (MAX_TEXT_UNITS + 1) * 2).unwrap());
            let ptr = GlobalLock(full.0).cast::<u16>();
            assert!(!ptr.is_null());
            let lock = super::super::Lock(full.0);
            let slice = std::slice::from_raw_parts_mut(ptr, MAX_TEXT_UNITS + 1);
            slice[..MAX_TEXT_UNITS].fill(b'a' as u16);
            slice[MAX_TEXT_UNITS] = 0;
            drop(lock);
            assert_eq!(read_block(full.0).unwrap().len(), MAX_TEXT_UNITS);
            assert_eq!(GlobalFlags(full.0) & 0xff, 0);
        }
    }
    #[test]
    fn strict_utf16_preserves_indentation_blank_lines_and_code_unit_boundary() {
        assert_eq!(
            parse("\t中文🦦 \r\n\r  code\n").unwrap(),
            "\t中文🦦 \n\n  code\n"
        );
        assert!(parse(&"🦦".repeat(MAX_TEXT_UNITS / 2)).is_ok());
        for value in [
            "\t\u{1} \r",
            "a\0b",
            &"a\n".repeat(400),
            &"a".repeat(MAX_TEXT_UNITS + 1),
        ] {
            assert!(parse(value).is_err());
        }
        assert!(normalize(&[0xd800]).is_err());
        assert!(parse(&"a\r\n".repeat(399)).is_ok());
    }
}
