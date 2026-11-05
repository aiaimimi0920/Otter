//! 有界原生窗口菜单；只读外部顶层窗口标题，选择绑定对象而非按标题再次查找。
use crate::live_window_target::Target;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
        System::Threading::GetCurrentProcessId,
        UI::WindowsAndMessaging::*,
    },
};
const LIMIT: usize = 64;
struct Candidate {
    target: Target,
    label: String,
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

fn label(title: &[u16], process: u32, hwnd: usize) -> String {
    let text = String::from_utf16_lossy(&title[..title.len().min(256)]);
    let mut clean = String::new();
    for character in text.chars().take(128) {
        match character {
            '&' => clean.push_str("&&"),
            c if c.is_control()
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') =>
            {
                clean.push(' ')
            }
            c => clean.push(c),
        }
    }
    let title = if clean.trim().is_empty() {
        "无标题窗口"
    } else {
        clean.trim()
    };
    format!("{title}  [PID {process} · {hwnd:X}]")
}
unsafe extern "system" fn enumerate(hwnd: HWND, parameter: LPARAM) -> windows::core::BOOL {
    let candidates = &mut *(parameter.0 as *mut Vec<Candidate>);
    if candidates.len() >= LIMIT || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return true.into();
    }
    let mut process = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process));
    // 不列出 Otter 自己，避免递归捕获；也避免向本进程窗口同步发送 WM_GETTEXT。
    if process == 0 || process == GetCurrentProcessId() {
        return true.into();
    }
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, (&mut cloaked as *mut u32).cast(), 4).is_err()
        || cloaked != 0
    {
        return true.into();
    }
    let Ok(target) = Target::bind(hwnd) else {
        return true.into();
    };
    let mut title = [0u16; 257];
    let length = GetWindowTextW(hwnd, &mut title).max(0) as usize;
    if target.validate().is_ok() {
        candidates.push(Candidate {
            target,
            label: label(&title[..length.min(256)], process, hwnd.0 as usize),
        });
    }
    true.into()
}
fn take(mut candidates: Vec<Candidate>, command: u32) -> Result<Option<Target>> {
    if command == 0 {
        return Ok(None);
    }
    let index = command as usize - 1;
    if index >= candidates.len() {
        return Err(Error::new(E_INVALIDARG, "Invalid Live window choice"));
    }
    let candidate = candidates.swap_remove(index);
    candidate.target.validate()?;
    Ok(Some(candidate.target))
}
unsafe fn menu(candidates: &[Candidate]) -> Result<Menu> {
    let menu = Menu(CreatePopupMenu()?);
    for (index, candidate) in candidates.iter().enumerate() {
        AppendMenuW(
            menu.0,
            MF_STRING,
            index + 1,
            &HSTRING::from(&candidate.label),
        )?;
    }
    if candidates.len() == LIMIT {
        AppendMenuW(menu.0, MF_SEPARATOR, 0, None)?;
        AppendMenuW(menu.0, MF_STRING | MF_GRAYED, 0, w!("仅显示前 64 个窗口"))?;
    }
    Ok(menu)
}
pub(super) unsafe fn select(owner: HWND) -> Result<Option<Target>> {
    let mut candidates = Vec::with_capacity(LIMIT);
    EnumWindows(
        Some(enumerate),
        LPARAM(&mut candidates as *mut Vec<Candidate> as isize),
    )?;
    if candidates.is_empty() {
        return Err(Error::new(E_FAIL, "No available external windows"));
    }
    let menu = menu(&candidates)?;
    let mut point = POINT::default();
    GetCursorPos(&mut point)?;
    let _ = SetForegroundWindow(owner);
    let command = TrackPopupMenu(
        menu.0,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        None,
        owner,
        None,
    )
    .0 as u32;
    let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
    take(candidates, command)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titles_are_bounded_and_cannot_inject_menu_shortcuts_or_control_layout() {
        let title: Vec<u16> = "A&B\tC\nD\u{202e}".encode_utf16().collect();
        assert_eq!(label(&title, 42, 16), "A&&B C D  [PID 42 · 10]");
        assert!(label(&[], 42, 16).starts_with("无标题窗口"));
        assert!(label(&vec![b'&' as u16; 10000], 1, 1).len() < 300);
        assert!(take(Vec::new(), 0).unwrap().is_none());
        assert!(take(Vec::new(), 1).is_err());
    }
    #[test]
    fn real_menu_choice_is_bound_to_hidden_fixture_and_closed_choice_is_rejected() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("fixture"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let window = crate::desktop::Window(hwnd);
            let first = Candidate {
                target: Target::bind(hwnd).unwrap(),
                label: "fixture".into(),
            };
            let stale = Candidate {
                target: Target::bind(hwnd).unwrap(),
                label: "fixture".into(),
            };
            let candidates = vec![first];
            let menu = menu(&candidates).unwrap();
            assert_eq!(GetMenuItemCount(Some(menu.0)), 1);
            assert_eq!(take(candidates, 1).unwrap().unwrap().hwnd(), hwnd);
            DestroyWindow(hwnd).unwrap();
            drop(window);
            assert!(take(vec![stale], 1).is_err());
        }
    }
}
