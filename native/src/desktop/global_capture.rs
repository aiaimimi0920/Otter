//! Built-in capture commands share one host. Dispatch only messages for a current registration.
use super::State;
#[path = "global_capture_registration.rs"]
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
mod registration;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use registration::Key;
use windows::{
    core::{w, Result, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
    },
};

pub(super) const COMMAND: usize = 0x1048;
#[cfg(test)]
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[cfg(test)]
const ID: i32 = 0x4f54;
#[cfg(test)]
const KEY: u32 = b'1' as u32;
#[cfg(test)]
const LONG_ID: i32 = 0x4f55;
#[cfg(test)]
const LONG_KEY: u32 = b'3' as u32;
#[cfg(test)]
const LIVE_ID: i32 = 0x4f56;
#[cfg(test)]
const LIVE_KEY: u32 = b'2' as u32;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
fn flags(state: &State) -> [&std::cell::Cell<bool>; 3] {
    [
        &state.global_capture,
        &state.global_live_capture,
        &state.global_long_capture,
    ]
}

pub(super) unsafe fn set(hwnd: HWND, state: &State, enabled: bool) -> Result<()> {
    super::shortcut_registry::configure(hwnd, state, enabled)
}
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) fn update(
    state: &State,
    enabled: bool,
    change: impl FnMut(Key, bool) -> Result<()>,
) -> Result<()> {
    let previous = state.global_capture.get();
    #[cfg(test)]
    super::shortcut_registry::fixture_bindings(state);
    let result = registration::set_with(flags(state), enabled, change);
    if previous || state.global_capture.get() {
        state
            .long_hotkey_enabled
            .set(state.global_long_capture.get());
        state
            .live_hotkey_enabled
            .set(state.global_live_capture.get());
    }
    if previous != state.global_capture.get() {
        state.hotkey_enabled.set(state.global_capture.get());
    }
    if let Some(error) = result? {
        eprintln!(
            "native-global-capture: optional shortcut unavailable (Ctrl+1 retained): {error}"
        );
    }
    Ok(())
}
pub(super) unsafe fn install(hwnd: HWND, state: &State) {
    if let Err(error) = super::shortcut_registry::resume(hwnd, state) {
        eprintln!("native-global-capture: shortcut registration failed: {error}");
    }
}
pub(super) unsafe fn toggle(hwnd: HWND, state: &State) {
    if let Err(error) = super::application_hotkey::request(hwnd, state) {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(crate::ui_text::format(
                "action-hotkey-failed",
                &[("error", error.to_string().into())],
            )),
            w!("Otter"),
            MB_OK | MB_ICONWARNING,
        );
    }
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING
            | if state.hotkey_enabled.get() {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        COMMAND,
        &HSTRING::from(label(state)),
    )
}
fn label(state: &State) -> String {
    let policy = state.shortcut_policy.borrow();
    let chords = crate::shortcut_policy::GLOBAL
        .iter()
        .map(|(id, _)| policy.chord(id).map(|chord| chord.label()))
        .collect::<Vec<_>>();
    let missing = chords
        .iter()
        .zip(
            state
                .shortcut_display
                .get()
                .map(|(_, slots)| slots)
                .unwrap_or([
                    true,
                    state.live_hotkey_enabled.get(),
                    state.long_hotkey_enabled.get(),
                    false,
                    false,
                    false,
                ]),
        )
        .filter_map(|(s, on)| if on { None } else { s.as_deref() })
        .collect::<Vec<_>>();
    let names = chords
        .iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" / ");
    let base = if state
        .shortcut_display
        .get()
        .is_some_and(|(suppressed, _)| suppressed)
    {
        crate::ui_text::text("action-hotkey-suspended").into()
    } else if state.hotkey_enabled.get() && !missing.is_empty() {
        crate::ui_text::format(
            "action-hotkey-degraded",
            &[("shortcuts", missing.join(" / ").into())],
        )
    } else {
        crate::ui_text::text("action-hotkey-menu").into()
    };
    if names.is_empty() {
        base
    } else {
        format!("{base}\t{names}")
    }
}
#[cfg(test)]
#[path = "global_capture_dispatch_tests.rs"]
mod dispatch_tests;
#[cfg(test)]
fn command(wp: WPARAM, lp: LPARAM) -> Option<usize> {
    super::shortcut_registry::match_binding(super::shortcut_registry::fixture_defaults(), wp, lp)
}

#[cfg(test)]
fn allowed(registered: bool, owner_enabled: bool, available: bool, wp: WPARAM, lp: LPARAM) -> bool {
    registered && owner_enabled && available && command(wp, lp).is_some()
}
pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    dispatch_with(hwnd, state, msg, wp, lp, |command| {
        if command == super::capture_input::live::COMMAND {
            super::capture_input::live::start(hwnd, state);
        } else {
            super::capture_input::start(hwnd, state, command);
        }
    })
}
unsafe fn dispatch_with(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    start: impl FnOnce(usize),
) -> Option<LRESULT> {
    if msg == WM_DESTROY {
        super::shortcut_registry::close(hwnd, state);
        return None;
    }
    if msg == super::fullscreen_watch::MESSAGE || msg == WM_DISPLAYCHANGE {
        super::fullscreen_watch::acknowledge(hwnd);
        super::shortcut_registry::refresh(hwnd, state);
        return (msg == super::fullscreen_watch::MESSAGE).then_some(LRESULT(0));
    }
    if msg != WM_HOTKEY {
        return None;
    }
    let Some(command) = super::shortcut_registry::route(state, wp, lp) else {
        return Some(LRESULT(0));
    };
    if command == super::pin_click_through::RESTORE {
        let mut p = POINT::default();
        if GetCursorPos(&mut p).is_ok() {
            state.click.signals.pointer.set(Some(p));
        } else {
            state.click.signals.all.set(true);
        }
        return Some(LRESULT(0));
    }
    if command == COMMAND {
        toggle(hwnd, state);
        return Some(LRESULT(0));
    }
    if command == super::capture_countdown::THREE {
        if !state.shortcut_host.suppressed.get()
            && IsWindowEnabled(hwnd).as_bool()
            && super::tray::recovery_installed(state)
        {
            if let Err(e) = super::capture_countdown::start(hwnd, state, command) {
                eprintln!("native-countdown-hotkey: {e}");
            }
        }
        return Some(LRESULT(0));
    }
    // Active scrolling capture exclusively owns capture and disables the original host; route finish requests first without new-selection admission.
    if command == super::capture_input::LONG_RECORDING
        && state.global_long_capture.get()
        && super::capture_input::finish_route::request(&state.capture_active)
    {
        return Some(LRESULT(0));
    }
    let ready = || {
        IsWindow(Some(hwnd)).as_bool()
            && super::capture_input::exclusive_available(state)
            && !state.shortcut_host.suppressed.get()
            && super::shortcut_registry::owns(state)
            && super::shortcut_registry::route(state, wp, lp) == Some(command)
            && match command {
                super::capture_input::LONG_RECORDING => state.global_long_capture.get(),
                super::capture_input::live::COMMAND => state.global_live_capture.get(),
                _ => state.global_capture.get(),
            }
            && IsWindowEnabled(hwnd).as_bool()
            && if command == super::capture_input::live::COMMAND {
                super::capture_input::live::available(state)
            } else {
                super::tool_panel_availability::Availability::capture(state)
                    .is_some_and(|availability| availability.allows(command))
            }
    };
    // Reuse the tested restore/reentry gate; capture owns hiding and cancellation.
    super::tray::actions::execute_with(
        command,
        ready,
        || super::tray::restore(hwnd, state),
        || start(command),
    );
    Some(LRESULT(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_secondary_status_tracks_degradation_without_non_owner_interference() {
        let owner = State::new(None, false).unwrap();
        let mut other = State::new(None, false).unwrap();
        other.hotkey_enabled = owner.hotkey_enabled.clone();
        other.long_hotkey_enabled = owner.long_hotkey_enabled.clone();
        other.live_hotkey_enabled = owner.live_hotkey_enabled.clone();
        update(&owner, true, |_, _| Ok(())).unwrap();
        update(&other, false, |_, _| panic!("non-owner unregister")).unwrap();
        assert!(
            other.hotkey_enabled.get()
                && other.long_hotkey_enabled.get()
                && other.live_hotkey_enabled.get()
        );
        let error = update(&owner, false, |key, enabled| {
            if key == Key::Primary || enabled {
                Err(windows::core::Error::new(E_FAIL, "fixture failure"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert!(error.message().contains("Ctrl+1 unregister failed"));
        assert!(error.message().contains("Ctrl+3 recovery failed"));
        assert!(error.message().contains("Ctrl+2 recovery failed"));
        assert!(other.hotkey_enabled.get() && !other.long_hotkey_enabled.get());
        assert!(owner.global_capture.get() && !owner.global_long_capture.get());
        update(&owner, false, |_, _| Ok(())).unwrap();
        assert!(!other.live_hotkey_enabled.get() && !owner.global_live_capture.get());
        update(&owner, true, |key, _| {
            if key == Key::Long {
                Err(windows::core::Error::new(E_FAIL, "occupied"))
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert!(other.hotkey_enabled.get() && !other.long_hotkey_enabled.get());
        assert!(other.live_hotkey_enabled.get());
        update(&owner, false, |_, _| Ok(())).unwrap();
    }
    #[test]
    fn disabled_owner_modal_or_busy_job_and_unregistered_state_reject_trigger() {
        let wp = WPARAM(ID as usize);
        let lp = LPARAM(((KEY << 16) | MOD_CONTROL.0) as isize);
        assert!(allowed(true, true, true, wp, lp));
        for (registered, enabled, available) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            assert!(!allowed(registered, enabled, available, wp, lp));
        }
    }
    #[test]
    fn message_requires_exact_id_key_and_modifiers() {
        let matches = |wp, lp| command(wp, lp).is_some();
        let payload = |key: u32, modifiers: u32| LPARAM(((key << 16) | modifiers) as isize);
        assert_eq!(
            command(WPARAM(0x4f56), payload(b'2' as u32, MOD_CONTROL.0)),
            Some(super::super::capture_input::live::COMMAND)
        );
        assert!(matches(WPARAM(ID as usize), payload(KEY, MOD_CONTROL.0)));
        assert!(matches(
            WPARAM(ID as usize),
            payload(KEY, MOD_CONTROL.0 | MOD_NOREPEAT.0)
        ));
        assert!(!matches(WPARAM(0), payload(KEY, MOD_CONTROL.0)));
        assert!(!matches(
            WPARAM(ID as usize),
            payload(b'2' as u32, MOD_CONTROL.0)
        ));
        for modifiers in [
            0,
            MOD_ALT.0,
            MOD_CONTROL.0 | MOD_SHIFT.0,
            MOD_CONTROL.0 | MOD_WIN.0,
        ] {
            assert!(!matches(WPARAM(ID as usize), payload(KEY, modifiers)));
            assert!(!matches(
                WPARAM(LONG_ID as usize),
                payload(LONG_KEY, modifiers)
            ));
            assert!(!matches(
                WPARAM(LIVE_ID as usize),
                payload(LIVE_KEY, modifiers)
            ));
        }
        assert_eq!(
            command(WPARAM(LONG_ID as usize), payload(LONG_KEY, MOD_CONTROL.0)),
            Some(super::super::capture_input::LONG_RECORDING)
        );
        assert_eq!(
            command(WPARAM(ID as usize), payload(KEY, MOD_CONTROL.0)),
            Some(super::super::capture_input::DEFAULT)
        );
        assert!(!matches(
            WPARAM(LONG_ID as usize),
            payload(KEY, MOD_CONTROL.0)
        ));
        assert!(!matches(
            WPARAM(ID as usize),
            payload(LONG_KEY, MOD_CONTROL.0)
        ));
    }
}
