//! Capture exclusion for documents and attached panels; roll back failures while retaining accurate state, and inherit requests when lazy panels are created.
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

pub(super) unsafe fn toggle(hwnd: HWND, state: &State) {
    let enabled = next_target(
        state.capture_protection.get(),
        state.capture_protection_requested.get(),
    );
    if let Err(error) = set(hwnd, state, enabled) {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(crate::ui_text::format(
                    "capture-protection-failed",
                    &[("error", error.to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

unsafe fn set(hwnd: HWND, state: &State, enabled: bool) -> Result<()> {
    state.capture_protection_requested.set(enabled);
    let windows = [
        hwnd,
        state.tool_panel.get(),
        state.parameters_ui.capture_window(),
        neuro_mini_ui::property_window(state.tool_panel.get()).unwrap_or_default(),
        super::pin_settings::window(state).unwrap_or_default(),
    ];
    apply(&state.capture_protection, enabled, |index, value| {
        if index >= 2 && windows[index].is_invalid() {
            return Ok(()); // Uncreated panels expose nothing; creation must explicitly inherit the request before allowing display.
        }
        SetWindowDisplayAffinity(
            windows[index],
            if value {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            },
        )
    })?;
    super::extension_notices::sync(state)
}

fn next_target<const N: usize>(actual: [bool; N], requested: bool) -> bool {
    if actual.iter().any(|flag| *flag != actual[0]) {
        requested
    } else {
        !actual[0]
    }
}

#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) unsafe fn smoke(hwnd: HWND, state: &State) -> Result<()> {
    set(hwnd, state, true)?;
    if state.capture_protection.get() != [true; 5] {
        return Err(Error::new(
            E_FAIL,
            "Capture protection enable state mismatch",
        ));
    }
    set(hwnd, state, false)?;
    if state.capture_protection.get() != [false; 5] {
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
