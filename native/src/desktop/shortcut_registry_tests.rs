//! Differential registration, stale delivery, and controlled Win32 contention without synthesizing user input.
use super::*;

fn chords() -> [Option<Chord>; 6] {
    fixture_defaults().map(|v| v.map(|(_, c)| c))
}

#[test]
fn swaps_release_first_and_queued_ids_cannot_dispatch_after_rebinding() {
    let mut actual = fixture_defaults();
    let old = actual[0].unwrap();
    let mut want = chords();
    want.swap(0, 2);
    let mut sequence = 500;
    transition(&mut actual, want, true, |i, target, bindings| {
        if let Some(c) = target {
            assert!(!bindings.iter().flatten().any(|(_, held)| *held == c));
            sequence += 1;
            bindings[i] = Some((sequence, c));
        } else {
            bindings[i] = None;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(actual.map(|v| v.map(|(_, c)| c)), want);
    let msg = LPARAM(((u32::from(old.1.key) << 16) | u32::from(old.1.modifiers)) as isize);
    assert_eq!(match_binding(actual, WPARAM(old.0 as usize), msg), None);
    assert_eq!(
        match_binding(actual, WPARAM(actual[2].unwrap().0 as usize), msg),
        Some(super::super::capture_input::LONG_RECORDING)
    );
}

#[test]
fn occupied_primary_aborts_edits_but_restore_registers_other_available_keys() {
    let mut actual = fixture_defaults();
    let mut want = chords();
    want[0].as_mut().unwrap().key = b'9';
    let apply = |i: usize, next: Option<Chord>, slots: &mut Slots| {
        if next.is_some_and(|c| c.key == b'9') {
            return Err(Error::new(E_ACCESSDENIED, "occupied"));
        }
        slots[i] = next.map(|c| (300 + i as i32, c));
        Ok(())
    };
    assert!(transition(&mut actual, want, true, apply).is_err());
    assert_eq!(actual.map(|v| v.map(|(_, c)| c)), chords());
    actual = [None; 6];
    transition(&mut actual, want, false, apply).unwrap();
    assert_eq!(
        actual.map(|v| v.is_some()),
        [false, true, true, false, false, false]
    );
}

#[test]
fn rollback_checks_previously_held_auxiliaries_without_requiring_suspended_or_unavailable_slots() {
    let before = chords();
    let mut after = before;
    after[2] = None;
    assert!(!restored(before, after, before));
    assert!(restored(before, after, after)); // the scroll-stop exemption expired
    assert!(restored(after, after, before)); // already occupied before the change
    after[2] = after[1];
    assert!(!restored(before, after, before));
}

#[test]
fn controls_survive_off_and_fullscreen_while_only_active_scroll_is_exempt() {
    let mut p = Policy::default();
    for (i, key) in [(3, b'6'), (4, b'7'), (5, b'8')] {
        p.bindings.push(crate::shortcut_policy::Binding {
            action: GLOBAL[i].0.into(),
            chord: Some(Chord { key, modifiers: 2 }),
        });
    }
    assert!(p.validate().is_ok());
    for (enabled, suppressed) in [(false, false), (false, true), (true, true)] {
        for scroll in [false, true] {
            assert_eq!(
                wanted(&p, true, enabled, suppressed, scroll).map(|c| c.is_some()),
                [false, false, scroll, false, true, true]
            );
            assert_eq!(wanted(&p, false, enabled, suppressed, scroll), [None; 6]);
        }
    }
}

#[test]
fn real_hotkey_rollback_releases_unsaved_keys_and_keeps_old_auxiliary() {
    use windows::{core::w, Win32::UI::WindowsAndMessaging::*};
    unsafe {
        let create = || {
            crate::desktop::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("Otter shortcut test"),
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
                .unwrap(),
            )
        };
        let (host, competitor) = (create(), create());
        let mods = MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT;
        // Keep the first chord with a different HWND. Probe for available
        // temporary combinations rather than unregistering another app's keys.
        let mut keys = Vec::new();
        for key in (0x70..=0x7au8).filter(|k| *k != 0x73) {
            let id = 800 + i32::from(key);
            if RegisterHotKey(Some(competitor.0), id, mods, u32::from(key)).is_ok() {
                if !keys.is_empty() {
                    UnregisterHotKey(Some(competitor.0), id).unwrap();
                }
                keys.push(Chord { key, modifiers: 7 });
                if keys.len() == 3 {
                    break;
                }
            }
        }
        assert_eq!(keys.len(), 3, "three temporary test chords required");
        let state = State::new(None, false).unwrap();
        state.hotkey_enabled.set(true);
        let policy = |primary, auxiliary| Policy {
            bindings: GLOBAL
                .iter()
                .enumerate()
                .map(|(i, (id, _))| crate::shortcut_policy::Binding {
                    action: (*id).into(),
                    chord: match i {
                        0 => Some(primary),
                        1 => auxiliary,
                        _ => None,
                    },
                })
                .collect(),
            ..Default::default()
        };
        let old = policy(keys[0], Some(keys[1]));
        state.shortcut_policy.replace(old.clone());
        resume(host.0, &state).unwrap();
        let before = held(&state);
        assert_eq!(&before[..2], &[None, Some(keys[1])]);
        state.shortcut_policy.replace(policy(keys[2], None));
        configure(host.0, &state, true).unwrap();
        state.shortcut_policy.replace(old);
        resume(host.0, &state).unwrap();
        assert!(recovered(&state, before));
        assert_eq!(held(&state), before);
        RegisterHotKey(Some(competitor.0), 1100, mods, keys[2].key.into()).unwrap();
        assert!(RegisterHotKey(Some(competitor.0), 1101, mods, keys[1].key.into()).is_err());
        close(host.0, &state);
    }
}
