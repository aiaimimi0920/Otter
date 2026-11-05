//! Ctrl+1 截图、Ctrl+2 Live 与 Ctrl+3 长截图；唯一宿主注册，无 hook 或轮询。
use super::State;
#[path = "global_capture_registration.rs"]
mod registration;
use registration::Key;
use windows::{
    core::{w, Result, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

pub(super) const COMMAND: usize = 0x1048;
const ID: i32 = 0x4f54;
const KEY: u32 = b'1' as u32;
const LONG_ID: i32 = 0x4f55;
const LONG_KEY: u32 = b'3' as u32;
const LIVE_ID: i32 = 0x4f56;
const LIVE_KEY: u32 = b'2' as u32;
fn binding(key: Key) -> (i32, u32) {
    match key {
        Key::Primary => (ID, KEY),
        Key::Live => (LIVE_ID, LIVE_KEY),
        Key::Long => (LONG_ID, LONG_KEY),
    }
}
fn flags(state: &State) -> [&std::cell::Cell<bool>; 3] {
    [
        &state.global_capture,
        &state.global_live_capture,
        &state.global_long_capture,
    ]
}

pub(super) unsafe fn set(hwnd: HWND, state: &State, enabled: bool) -> Result<()> {
    update(state, enabled, |key, enabled| {
        let (id, key) = binding(key);
        if enabled {
            RegisterHotKey(Some(hwnd), id, MOD_CONTROL | MOD_NOREPEAT, key)
        } else {
            UnregisterHotKey(Some(hwnd), id)
        }
    })
}
pub(super) fn update(
    state: &State,
    enabled: bool,
    change: impl FnMut(Key, bool) -> Result<()>,
) -> Result<()> {
    let previous = state.global_capture.get();
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
    if let Err(error) = set(hwnd, state, true) {
        eprintln!("native-global-capture: Ctrl+1 unavailable: {error}");
    }
}
pub(super) unsafe fn toggle(hwnd: HWND, state: &State) {
    if let Err(error) = super::application_hotkey::request(hwnd, state) {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!(
                "无法切换 Ctrl+1 全局截图快捷键。可能已被其他程序占用。\n{error}"
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
    let missing: Vec<_> = [
        ("Ctrl+2", state.live_hotkey_enabled.get()),
        ("Ctrl+3", state.long_hotkey_enabled.get()),
    ]
    .into_iter()
    .filter_map(|(label, enabled)| (!enabled).then_some(label))
    .collect();
    if state.hotkey_enabled.get() && !missing.is_empty() {
        format!(
            "全局截图快捷键（{} 不可用）\tCtrl+1 / Ctrl+2 / Ctrl+3",
            missing.join(" / ")
        )
    } else {
        "全局截图 / Live / 长截图录制\tCtrl+1 / Ctrl+2 / Ctrl+3".into()
    }
}
#[cfg(test)]
#[path = "global_capture_dispatch_tests.rs"]
mod dispatch_tests;
fn command(wp: WPARAM, lp: LPARAM) -> Option<usize> {
    let value = lp.0 as u32;
    if (value & 0xffff & !MOD_NOREPEAT.0) != MOD_CONTROL.0 {
        return None;
    }
    match (wp.0, value >> 16) {
        (id, KEY) if id == ID as usize => Some(super::capture_input::DEFAULT),
        (id, LONG_KEY) if id == LONG_ID as usize => Some(super::capture_input::LONG_RECORDING),
        (id, LIVE_KEY) if id == LIVE_ID as usize => Some(super::capture_input::live::COMMAND),
        _ => None,
    }
}

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
        // Destruction must attempt every release, even if the first one fails.
        let owned = state.global_capture.get();
        for error in registration::cleanup_with(flags(state), |key| {
            UnregisterHotKey(Some(hwnd), binding(key).0)
        })
        .into_iter()
        .flatten()
        {
            eprintln!("native-global-capture-cleanup: {error}");
        }
        if owned {
            state.hotkey_enabled.set(false);
            state.long_hotkey_enabled.set(false);
            state.live_hotkey_enabled.set(false);
        }
        return None;
    }
    if msg != WM_HOTKEY {
        return None;
    }
    let Some(command) = command(wp, lp) else {
        return Some(LRESULT(0));
    };
    // 活跃长截图独占采集且禁用了原宿主；先路由结束请求，不重走新选区准入。
    if command == super::capture_input::LONG_RECORDING
        && state.global_capture.get()
        && state.global_long_capture.get()
        && super::capture_input::finish_route::request(&state.capture_active)
    {
        return Some(LRESULT(0));
    }
    let ready = || {
        IsWindow(Some(hwnd)).as_bool()
            && super::capture_input::exclusive_available(state)
            && allowed(
                state.global_capture.get()
                    && match command {
                        super::capture_input::LONG_RECORDING => state.global_long_capture.get(),
                        super::capture_input::live::COMMAND => state.global_live_capture.get(),
                        _ => true,
                    },
                IsWindowEnabled(hwnd).as_bool(),
                if command == super::capture_input::live::COMMAND {
                    super::capture_input::live::available(state)
                } else {
                    super::tool_panel_availability::Availability::capture(state)
                        .is_some_and(|availability| availability.allows(command))
                },
                wp,
                lp,
            )
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
