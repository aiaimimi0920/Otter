//! Per-window ownership of registered chords; an unbound or suspended host still owns the switch.
use super::{fullscreen_watch, State};
use crate::shortcut_policy::{Chord, Policy, GLOBAL};
use std::cell::{Cell, RefCell};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::Input::KeyboardAndMouse::*},
};

#[derive(Default)]
pub(super) struct Host {
    attached: Cell<bool>,
    scrolling: Cell<bool>,
    pub suppressed: Cell<bool>,
    pub watch: RefCell<Option<fullscreen_watch::Watch>>,
    actual: Cell<Slots>,
    #[cfg(test)]
    fixture: Cell<bool>,
}
type Slots = [Option<(i32, Chord)>; 6];
pub(super) type Chords = [Option<Chord>; 6];
pub(super) fn held(state: &State) -> Chords {
    state.shortcut_host.actual.get().map(|b| b.map(|(_, c)| c))
}
pub(super) unsafe fn recovered(state: &State, before: Chords) -> bool {
    let desired = wanted(
        &state.shortcut_policy.borrow(),
        true,
        state.hotkey_enabled.get(),
        state.shortcut_host.suppressed.get(),
        super::capture_input::finish_route::active(&state.capture_active),
    );
    restored(before, held(state), desired)
}
fn restored(before: Chords, after: Chords, desired: Chords) -> bool {
    (0..before.len()).all(|i| {
        // A concurrent fullscreen or master-switch transition may legitimately
        // remove a key. An occupied previously held key is not a successful rollback.
        after[i].is_none_or(|c| Some(c) == desired[i])
            && (before[i].is_none() || before[i] != desired[i] || before[i] == after[i])
    })
}
pub(super) fn registered(state: &State) -> [bool; 6] {
    state.shortcut_host.actual.get().map(|b| b.is_some())
}

thread_local! {
    // IDs are never recycled while old messages can still be queued on the UI thread.
    static NEXT: Cell<i32> = const { Cell::new(0x100) };
    static HOST: Cell<Option<HWND>> = const {Cell::new(None)};
}
fn allocate() -> Result<i32> {
    NEXT.with(|counter| {
        let id = counter.get();
        if id >= 0xbfff {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Built-in shortcut ID budget exhausted",
            ));
        }
        counter.set(id + 1);
        Ok(id)
    })
}

pub(super) fn owns(state: &State) -> bool {
    state.shortcut_host.attached.get() || state.global_capture.get()
}
fn command(i: usize) -> usize {
    [
        super::capture_input::DEFAULT,
        super::capture_input::live::COMMAND,
        super::capture_input::LONG_RECORDING,
        super::capture_countdown::THREE,
        super::pin_click_through::RESTORE,
        super::global_capture::COMMAND,
    ][i]
}
pub(super) fn route(state: &State, wp: WPARAM, lp: LPARAM) -> Option<usize> {
    match_binding(state.shortcut_host.actual.get(), wp, lp)
}
pub(super) fn match_binding(actual: Slots, wp: WPARAM, lp: LPARAM) -> Option<usize> {
    let payload = lp.0 as u32;
    actual.into_iter().enumerate().find_map(|(i, b)| {
        let (id, chord) = b?;
        (wp.0 == id as usize
            && payload >> 16 == u32::from(chord.key)
            && payload & 0xffff & !MOD_NOREPEAT.0 == u32::from(chord.modifiers))
        .then(|| command(i))
    })
}

#[cfg(test)]
pub(super) fn fixture_bindings(state: &State) {
    state.shortcut_host.fixture.set(true);
    state.shortcut_host.actual.set(fixture_defaults());
}
#[cfg(test)]
pub(super) fn fixture_defaults() -> Slots {
    std::array::from_fn(|i| {
        if i >= 3 {
            return None;
        }
        Some((
            [0x4f54, 0x4f56, 0x4f55][i],
            Chord {
                key: GLOBAL[i].1,
                modifiers: crate::shortcut_policy::CTRL,
            },
        ))
    })
}

unsafe fn reconcile(
    hwnd: HWND,
    state: &State,
    policy: &Policy,
    attached: bool,
    enabled: bool,
    suppressed: bool,
    strict_primary: bool,
) -> Result<()> {
    let scrolling = super::capture_input::finish_route::active(&state.capture_active);
    let wanted = wanted(policy, attached, enabled, suppressed, scrolling);
    let before = state.shortcut_host.actual.get();
    let mut actual = before;
    let apply = |i: usize, target: Option<Chord>, actual: &mut Slots| -> Result<()> {
        if let Some((id, _)) = actual[i] {
            UnregisterHotKey(Some(hwnd), id)?;
            actual[i] = None;
        }
        if let Some(chord) = target {
            let id = allocate()?;
            RegisterHotKey(
                Some(hwnd),
                id,
                HOT_KEY_MODIFIERS(chord.modifiers.into()) | MOD_NOREPEAT,
                chord.key.into(),
            )?;
            actual[i] = Some((id, chord));
        }
        Ok(())
    };
    let result = transition(&mut actual, wanted, strict_primary, apply);
    state.shortcut_host.actual.set(actual);
    state.global_capture.set(actual[0].is_some());
    state.global_live_capture.set(actual[1].is_some());
    state.global_long_capture.set(actual[2].is_some());
    if state.shortcut_host.attached.get() || attached {
        state
            .shortcut_display
            .set(Some((suppressed, actual.map(|s| s.is_some()))));
        state.live_hotkey_enabled.set(actual[1].is_some());
        state.long_hotkey_enabled.set(actual[2].is_some());
    }
    result
}

fn wanted(
    policy: &Policy,
    attached: bool,
    enabled: bool,
    suppressed: bool,
    scrolling: bool,
) -> [Option<Chord>; 6] {
    std::array::from_fn(|i| {
        (attached && (i >= 4 || (i == 2 && scrolling) || (enabled && !suppressed)))
            .then(|| policy.chord(GLOBAL[i].0))
            .flatten()
    })
}

fn transition(
    actual: &mut Slots,
    wanted: [Option<Chord>; 6],
    strict_primary: bool,
    mut apply: impl FnMut(usize, Option<Chord>, &mut Slots) -> Result<()>,
) -> Result<()> {
    let before = actual.map(|v| v.map(|(_, c)| c));
    let result = (|| {
        // Release obsolete chords before additions so swapping two actions is legal.
        for i in [1, 2, 3, 4, 5, 0] {
            if before[i].is_some() && before[i] != wanted[i] {
                apply(i, None, actual)?;
            }
        }
        for i in 0..actual.len() {
            if let Some(chord) = wanted[i].filter(|_| actual[i].is_none()) {
                if let Err(error) = apply(i, Some(chord), actual) {
                    if i == 0 && strict_primary {
                        return Err(error);
                    }
                    eprintln!("native-shortcut-unavailable: {}: {error}", GLOBAL[i].0);
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut failures = vec![error.to_string()];
        for i in 0..actual.len() {
            if actual[i].is_none() && before[i].is_some() {
                if let Err(e) = apply(i, before[i], actual) {
                    failures.push(format!("restore {}: {e}", GLOBAL[i].0));
                }
            }
        }
        return Err(Error::new(error.code(), failures.join("; ")));
    }
    Ok(())
}

pub(super) unsafe fn configure(hwnd: HWND, state: &State, enabled: bool) -> Result<()> {
    configure_with(hwnd, state, true, enabled, true)?;
    state.hotkey_enabled.set(enabled);
    Ok(())
}
// Startup and host transfer restore an existing policy; one occupied key must
// not disable the others or silently flip the user's master switch.
pub(super) unsafe fn resume(hwnd: HWND, state: &State) -> Result<()> {
    configure_with(hwnd, state, true, state.hotkey_enabled.get(), false)
}
pub(super) unsafe fn transfer(hwnd: HWND, state: &State, attach: bool) -> Result<()> {
    configure_with(hwnd, state, attach, state.hotkey_enabled.get(), false)
}
unsafe fn configure_with(
    hwnd: HWND,
    state: &State,
    attach: bool,
    enabled: bool,
    strict: bool,
) -> Result<()> {
    if !attach && !owns(state) {
        return Ok(());
    }
    if attach && HOST.with(Cell::get).is_some_and(|h| h != hwnd) {
        return Err(Error::new(E_UNEXPECTED, "Duplicate shortcut host"));
    }
    let policy = state.shortcut_policy.borrow().clone();
    let watch = attach && enabled && policy.fullscreen.enabled;
    let new_watch = if watch && state.shortcut_host.watch.borrow().is_none() {
        match fullscreen_watch::Watch::install(hwnd) {
            Ok(w) => Some(w),
            Err(error) if !strict => {
                eprintln!(
                    "native-fullscreen-watch-unavailable: {error}; capture shortcuts suspended"
                );
                None
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let suppressed = watch
        && ((new_watch.is_none() && state.shortcut_host.watch.borrow().is_none())
            || fullscreen_watch::matches(policy.fullscreen.scope));
    reconcile(hwnd, state, &policy, attach, enabled, suppressed, strict)?;
    if let Some(new_watch) = new_watch {
        state.shortcut_host.watch.replace(Some(new_watch));
    }
    if !watch {
        state.shortcut_host.watch.borrow_mut().take();
    }
    HOST.with(|h| h.set(attach.then_some(hwnd)));
    state.shortcut_host.attached.set(attach);
    state
        .shortcut_host
        .scrolling
        .set(attach && super::capture_input::finish_route::active(&state.capture_active));
    state.shortcut_host.suppressed.set(suppressed);
    if !attach {
        state.shortcut_display.set(None);
    }
    Ok(())
}

pub(super) unsafe fn refresh(hwnd: HWND, state: &State) {
    if !state.shortcut_host.attached.get() {
        return;
    }
    let policy = state.shortcut_policy.borrow().clone();
    let suppressed = state.hotkey_enabled.get()
        && policy.fullscreen.enabled
        && (state.shortcut_host.watch.borrow().is_none()
            || fullscreen_watch::matches(policy.fullscreen.scope));
    let changed = state.shortcut_host.suppressed.get() != suppressed;
    let scrolling = super::capture_input::finish_route::active(&state.capture_active);
    let stop_changed = state.shortcut_host.scrolling.get() != scrolling;
    if changed || stop_changed {
        if let Err(error) = reconcile(
            hwnd,
            state,
            &policy,
            true,
            state.hotkey_enabled.get(),
            suppressed,
            false,
        ) {
            eprintln!("native-shortcut-restore: {error}");
        } else {
            state.shortcut_host.suppressed.set(suppressed);
            state.shortcut_host.scrolling.set(scrolling);
        }
    }
}

pub(super) unsafe fn finish_changed() {
    if let Some(hwnd) = HOST.with(Cell::get) {
        if let Err(error) = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(hwnd),
            fullscreen_watch::MESSAGE,
            WPARAM(0),
            LPARAM(0),
        ) {
            eprintln!("native-shortcut-finish-wake: {error}");
        }
    }
}

pub(super) unsafe fn close(hwnd: HWND, state: &State) {
    state.shortcut_host.watch.borrow_mut().take();
    #[cfg(test)]
    if state.shortcut_host.fixture.replace(false) {
        state.shortcut_host.actual.take();
    }
    for (id, _) in state.shortcut_host.actual.take().into_iter().flatten() {
        if let Err(error) = UnregisterHotKey(Some(hwnd), id) {
            eprintln!("native-shortcut-cleanup: {error}");
        }
    }
    if state.shortcut_host.attached.replace(false) {
        HOST.with(|host| {
            if host.get() == Some(hwnd) {
                host.set(None)
            }
        });
        state.shortcut_display.set(None);
        state.long_hotkey_enabled.set(false);
        state.live_hotkey_enabled.set(false);
        state.global_capture.set(false);
        state.global_live_capture.set(false);
        state.global_long_capture.set(false);
    }
}

#[cfg(test)]
#[path = "shortcut_registry_tests.rs"]
mod tests;
