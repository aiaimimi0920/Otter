//! Read Unicode text once only for explicit Number paste; copy bounded content while locked without retaining system handles.
use super::super::super::surface_edit_control::MAX_UNITS;
use windows::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*},
};
struct Clipboard;
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}
struct Lock(HGLOBAL);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.0);
        }
    }
}
fn decode(bytes: &[u8]) -> Option<Vec<u16>> {
    if bytes.len() < 2 || bytes.len() > (MAX_UNITS + 1) * 2 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut result = Vec::new();
    for unit in bytes
        .chunks_exact(2)
        .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
    {
        if unit == 0 {
            return Some(result);
        }
        if result.len() == MAX_UNITS {
            return None;
        }
        result.push(unit);
    }
    None
}
pub unsafe fn read(hwnd: HWND) -> Option<Vec<u16>> {
    OpenClipboard(Some(hwnd)).ok()?;
    let _clipboard = Clipboard;
    let handle = GetClipboardData(13).ok()?; // CF_UNICODETEXT, including system-provided text-format conversion.
    let memory = HGLOBAL(handle.0);
    let size = GlobalSize(memory);
    if !(2..=(MAX_UNITS + 1) * 2).contains(&size) {
        return None;
    }
    let pointer = GlobalLock(memory);
    if pointer.is_null() {
        return None;
    }
    let _lock = Lock(memory); // Still system-owned; GlobalFree is forbidden.
    decode(std::slice::from_raw_parts(pointer.cast::<u8>(), size))
}
#[test]
fn surface_number_clipboard_decodes_only_bounded_terminated_utf16() {
    assert_eq!(
        decode(&[b'1', 0, b'.', 0, b'5', 0, 0, 0]),
        Some(vec![49, 46, 53])
    );
    assert_eq!(decode(&[0, 0]), Some(vec![]));
    assert!(decode(&[49, 0]).is_none());
    assert!(decode(&[49, 0, 0]).is_none());
    assert!(decode(&vec![0; (MAX_UNITS + 2) * 2]).is_none());
    // reader handles only format/capacity; later preflight rejects invalid numeric grammar and malformed surrogates.
    assert_eq!(decode(&[0, 0xd8, 0, 0]), Some(vec![0xd800]));
}
