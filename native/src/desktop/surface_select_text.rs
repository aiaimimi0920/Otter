//! Bounded reads from the system string table; collapsed areas/lists share text without another selection state.
use super::*;
pub(super) unsafe fn at(hwnd: HWND, index: usize) -> Option<Vec<u16>> {
    if index == usize::MAX || index == u32::MAX as usize {
        return Some(Vec::new());
    }
    let length = SendMessageW(hwnd, CB_GETLBTEXTLEN, Some(WPARAM(index)), None).0;
    if !(0..=48 * 1024).contains(&length) {
        return None;
    }
    let mut text = vec![0; length as usize + 1];
    let actual = SendMessageW(
        hwnd,
        CB_GETLBTEXT,
        Some(WPARAM(index)),
        Some(LPARAM(text.as_mut_ptr() as isize)),
    )
    .0;
    if actual != length {
        return None;
    }
    text.truncate(length as usize);
    Some(text)
}
