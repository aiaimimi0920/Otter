//! Reject predictable Number insertion/deletion before EDIT mutation, preserving system undo/selection without clipboard reads.
use super::super::surface_edit_control::{MAX_BYTES, MAX_UNITS};
use windows::Win32::{
    Foundation::*,
    UI::{Controls::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

fn allowed_unit(unit: u16) -> bool {
    matches!(unit, 0x30..=0x39 | 0x2b | 0x2d | 0x2e | 0x45 | 0x65)
}
unsafe fn replacement(lp: LPARAM) -> Option<Vec<u16>> {
    if lp.0 == 0 {
        return None;
    }
    // By Win32 contract, EM_REPLACESEL carries a NUL-terminated UTF-16 pointer valid throughout synchronous handling.
    // Do not retain the pointer; inspect at most MAX_UNITS + 1 units. This is not a generic external-memory reader.
    let pointer = lp.0 as *const u16;
    let mut text = Vec::new();
    for index in 0..=MAX_UNITS {
        let unit = pointer.add(index).read();
        if unit == 0 {
            return Some(text);
        }
        if index == MAX_UNITS || !allowed_unit(unit) {
            return None;
        }
        text.push(unit);
    }
    None
}
fn candidate(text: &[u16], selection: (usize, usize), replacement: &[u16]) -> bool {
    let (start, end) = selection;
    if start > end
        || end > text.len()
        || text
            .iter()
            .chain(replacement)
            .any(|unit| !allowed_unit(*unit))
    {
        return false;
    }
    let size = text.len() - (end - start) + replacement.len();
    // ASCII numeric-edit syntax only, so UTF-8 byte counts equal UTF-16 unit counts.
    if size > MAX_UNITS.min(MAX_BYTES) {
        return false;
    }
    let value: String = text[..start]
        .iter()
        .chain(replacement)
        .chain(&text[end..])
        .map(|unit| *unit as u8 as char)
        .collect();
    crate::surface_number::editable(&value)
}
#[derive(Clone, Copy)]
enum Erase {
    Selection,
    Forward,
    Backward(usize),
}
fn erase_range(kind: Erase, range: (usize, usize), length: usize) -> Option<(usize, usize)> {
    let (start, end) = range;
    if start > end || end > length {
        return None;
    }
    Some(match kind {
        Erase::Selection => range,
        Erase::Forward if start == end => (start, (end + 1).min(length)),
        Erase::Forward => range,
        // Classic EDIT Backspace first deletes selection; remaining repeats delete backward.
        Erase::Backward(count) => (
            start.saturating_sub(count.saturating_sub(usize::from(start != end))),
            end,
        ),
    })
}
unsafe fn unmodified() -> bool {
    [VK_CONTROL, VK_MENU, VK_SHIFT]
        .iter()
        .all(|key| GetKeyState(key.0 as i32) >= 0)
}
pub unsafe fn rejects(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> bool {
    let erase = match message {
        WM_CLEAR | WM_CUT => Some(Erase::Selection),
        WM_KEYDOWN if wp.0 == VK_DELETE.0 as usize && unmodified() => Some(Erase::Forward),
        WM_CHAR if wp.0 == VK_BACK.0 as usize && unmodified() => {
            Some(Erase::Backward(((lp.0 as usize) & 0xffff).max(1)))
        }
        _ => None,
    };
    let replacement = if erase.is_some() {
        Some(Vec::new())
    } else if message == EM_REPLACESEL {
        replacement(lp)
    } else if message == WM_CHAR && wp.0 >= 0x20 {
        u16::try_from(wp.0)
            .ok()
            .filter(|unit| allowed_unit(*unit))
            // Classic EDIT inserts according to WM_CHAR's low-16-bit repeat count; zero remains compatible with existing single-character messages.
            .map(|unit| vec![unit; ((lp.0 as usize) & 0xffff).max(1)])
    } else {
        return false;
    };
    let Some(replacement) = replacement else {
        return true;
    };
    let length = GetWindowTextLengthW(hwnd);
    if length < 0 || length as usize > MAX_UNITS {
        return true;
    }
    let mut text = vec![0_u16; length as usize + 1];
    if GetWindowTextW(hwnd, &mut text) != length {
        return true;
    }
    text.truncate(length as usize);
    let mut start = 0_u32;
    let mut end = 0_u32;
    SendMessageW(
        hwnd,
        EM_GETSEL,
        Some(WPARAM((&mut start as *mut u32) as usize)),
        Some(LPARAM((&mut end as *mut u32) as isize)),
    );
    let range = (start as usize, end as usize);
    let range = if let Some(erase) = erase {
        erase_range(erase, range, text.len())
    } else {
        Some(range)
    };
    !range.is_some_and(|range| candidate(&text, range, &replacement))
}
#[test]
fn surface_number_preflight_candidate_selection_and_budget() {
    let text: Vec<_> = "12.5".encode_utf16().collect();
    assert!(candidate(&text, (1, 3), &[b'3' as u16]));
    assert!(!candidate(&text, (4, 4), &[b'.' as u16]));
    assert!(candidate(&text, (0, 4), &[]));
    assert!(!candidate(&text, (3, 2), &[]));
    assert!(!candidate(&text, (0, 5), &[]));
    assert!(!candidate(
        &vec![b'1' as u16; MAX_UNITS],
        (0, 0),
        &[b'2' as u16]
    ));
}

#[test]
fn surface_number_preflight_erase_ranges_and_cut_candidate() {
    assert_eq!(erase_range(Erase::Forward, (4, 4), 4), Some((4, 4)));
    assert_eq!(erase_range(Erase::Backward(9), (2, 2), 4), Some((0, 2)));
    assert_eq!(erase_range(Erase::Backward(2), (1, 3), 4), Some((0, 3)));
    assert_eq!(erase_range(Erase::Selection, (2, 2), 4), Some((2, 2)));
    assert_eq!(erase_range(Erase::Selection, (3, 2), 4), None);
    assert_eq!(erase_range(Erase::Forward, (5, 5), 4), None);
    let text: Vec<_> = "1e+2".encode_utf16().collect();
    // WM_CUT and WM_CLEAR share selection candidates; calculate only the candidate here without sending cut messages.
    let range = erase_range(Erase::Selection, (1, 2), text.len()).unwrap();
    assert!(!candidate(&text, range, &[]));
    let range = erase_range(Erase::Selection, (1, 3), text.len()).unwrap();
    assert!(candidate(&text, range, &[]));
}
