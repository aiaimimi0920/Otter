//! 文档与两个独立面板的捕获排除；失败回滚并保留准确状态，惰性面板创建时继承请求。
use super::State;
use std::cell::Cell;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) const COMMAND: usize = 0x1046;

fn apply<const N: usize>(
    state: &Cell<[bool; N]>,
    enabled: bool,
    mut set: impl FnMut(usize, bool) -> Result<()>,
) -> Result<()> {
    let original = state.get();
    let mut changed = Vec::with_capacity(N);
    for index in 0..N {
        if original[index] == enabled {
            continue;
        }
        if let Err(error) = set(index, enabled) {
            let mut failures = Vec::new();
            for index in changed.into_iter().rev() {
                match set(index, original[index]) {
                    Ok(()) => {
                        let mut actual = state.get();
                        actual[index] = original[index];
                        state.set(actual);
                    }
                    Err(rollback) => failures.push(rollback.to_string()),
                }
            }
            return if failures.is_empty() {
                Err(error)
            } else {
                Err(Error::new(
                    E_FAIL,
                    format!("{error}; rollback failed: {}", failures.join("; ")),
                ))
            };
        }
        let mut actual = state.get();
        actual[index] = enabled;
        state.set(actual);
        changed.push(index);
    }
    Ok(())
}

pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let flags = state.capture_protection.get();
    let label = if flags.iter().any(|flag| *flag != flags[0]) {
        if state.capture_protection_requested.get() {
            w!("捕获排除（部分生效，重试开启）")
        } else {
            w!("捕获排除（部分生效，重试关闭）")
        }
    } else {
        w!("从系统截图中排除此窗口与面板")
    };
    AppendMenuW(
        menu,
        MF_STRING
            | if flags == [true; 3] {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        COMMAND,
        label,
    )
}

pub(super) unsafe fn toggle(hwnd: HWND, state: &State) {
    let enabled = next_target(
        state.capture_protection.get(),
        state.capture_protection_requested.get(),
    );
    if let Err(error) = set(hwnd, state, enabled) {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(format!("无法完整设置捕获排除。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

unsafe fn set(hwnd: HWND, state: &State, enabled: bool) -> Result<()> {
    state.capture_protection_requested.set(enabled);
    let windows = [hwnd, state.tool_panel.get(), state.parameters_ui.window()];
    apply(&state.capture_protection, enabled, |index, value| {
        if index == 2 && windows[index].is_invalid() {
            return Ok(()); // 尚未创建的面板没有泄露面；创建必须显式继承请求才允许显示。
        }
        SetWindowDisplayAffinity(
            windows[index],
            if value {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            },
        )
    })
}

fn next_target<const N: usize>(actual: [bool; N], requested: bool) -> bool {
    if actual.iter().any(|flag| *flag != actual[0]) {
        requested
    } else {
        !actual[0]
    }
}

pub(super) unsafe fn smoke(hwnd: HWND, state: &State) -> Result<()> {
    set(hwnd, state, true)?;
    if state.capture_protection.get() != [true; 3] {
        return Err(Error::new(
            E_FAIL,
            "Capture protection enable state mismatch",
        ));
    }
    set(hwnd, state, false)?;
    if state.capture_protection.get() != [false; 3] {
        return Err(Error::new(
            E_FAIL,
            "Capture protection disable state mismatch",
        ));
    }
    println!(
        "native-capture-protection-smoke: host=true panel=true reset=true desktop-access=false"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_retry_preserves_requested_direction_including_disable() {
        for actual in [[false, true], [true, false]] {
            assert!(next_target(actual, true));
            assert!(!next_target(actual, false));
        }
        assert!(next_target([false; 2], false));
        assert!(!next_target([true; 2], true));
    }
    #[test]
    fn first_failure_does_not_touch_other_window_or_claim_success() {
        for original in [[false; 2], [true; 2]] {
            let state = Cell::new(original);
            let mut calls = 0;
            assert!(apply(&state, !original[0], |index, _| {
                assert_eq!(index, 0);
                calls += 1;
                Err(Error::new(E_FAIL, "first window"))
            })
            .is_err());
            assert_eq!(calls, 1);
            assert_eq!(state.get(), original);
        }
    }
    #[test]
    fn changes_only_required_windows_and_skips_repeated_state() {
        let state = Cell::new([false; 2]);
        let mut calls = Vec::new();
        for enabled in [true, true, false, false] {
            apply(&state, enabled, |index, value| {
                calls.push((index, value));
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(calls, [(0, true), (1, true), (0, false), (1, false)]);
        assert_eq!(state.get(), [false; 2]);
    }
    #[test]
    fn second_window_failure_rolls_back_first() {
        let state = Cell::new([false; 2]);
        let mut calls = Vec::new();
        assert!(apply(&state, true, |index, value| {
            calls.push((index, value));
            if index == 1 {
                Err(Error::new(E_FAIL, "second window"))
            } else {
                Ok(())
            }
        })
        .is_err());
        assert_eq!(calls, [(0, true), (1, true), (0, false)]);
        assert_eq!(state.get(), [false; 2]);
    }
    #[test]
    fn rollback_failure_preserves_partial_state_and_retry_only_fills_gap() {
        let state = Cell::new([false; 2]);
        let result = apply(&state, true, |index, value| {
            if index == 1 || !value {
                Err(Error::new(E_FAIL, "injected"))
            } else {
                Ok(())
            }
        });
        assert!(result.unwrap_err().to_string().contains("rollback failed"));
        assert_eq!(state.get(), [true, false]);
        let mut calls = Vec::new();
        apply(&state, true, |index, value| {
            calls.push((index, value));
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, [(1, true)]);
        assert_eq!(state.get(), [true; 2]);
    }
}
