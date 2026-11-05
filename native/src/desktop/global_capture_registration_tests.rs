//! 三键实际状态与补偿矩阵；注入系统适配器，不注册用户全局快捷键。
use super::*;
use windows::Win32::Foundation::E_FAIL;
use Key::{Live, Long, Primary};
fn failure() -> Error {
    Error::new(E_FAIL, "fixture registration failure")
}
fn refs(flags: &[Cell<bool>; 3]) -> [&Cell<bool>; 3] {
    [&flags[0], &flags[1], &flags[2]]
}
fn actual(flags: &[Cell<bool>; 3]) -> [bool; 3] {
    flags.each_ref().map(Cell::get)
}
fn flags(mask: u8) -> [Cell<bool>; 3] {
    [0, 1, 2].map(|bit| Cell::new(mask & (1 << bit) != 0))
}
#[test]
fn optional_conflicts_are_independent_and_successful_registrations_are_not_repeated() {
    for conflicts in 0u8..4 {
        let flags = flags(0);
        let mut calls = Vec::new();
        let warning = set_with(refs(&flags), true, |key, enabled| {
            calls.push((key, enabled));
            if (key == Live && conflicts & 1 != 0) || (key == Long && conflicts & 2 != 0) {
                Err(failure())
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(calls, [(Primary, true), (Live, true), (Long, true)]);
        assert_eq!(
            actual(&flags),
            [true, conflicts & 1 == 0, conflicts & 2 == 0]
        );
        assert_eq!(warning.is_some(), conflicts != 0);
        if let Some(warning) = warning {
            assert_eq!(warning.message().contains("Ctrl+2"), conflicts & 1 != 0);
            assert_eq!(warning.message().contains("Ctrl+3"), conflicts & 2 != 0);
        }
        calls.clear();
        set_with(refs(&flags), true, |key, enabled| {
            calls.push((key, enabled));
            Ok(())
        })
        .unwrap();
        assert_eq!(calls.len(), conflicts.count_ones() as usize);
        assert_eq!(actual(&flags), [true; 3]);
        set_with(refs(&flags), true, |_, _| panic!("duplicate registration")).unwrap();
        set_with(refs(&flags), false, |_, enabled| {
            assert!(!enabled);
            Ok(())
        })
        .unwrap();
        assert_eq!(actual(&flags), [false; 3]);
        set_with(refs(&flags), false, |_, _| panic!("duplicate unregister")).unwrap();
    }
}
#[test]
fn primary_registration_failure_never_attempts_or_owns_secondary_keys() {
    let flags = flags(0);
    assert!(set_with(refs(&flags), true, |key, enabled| {
        assert_eq!((key, enabled), (Primary, true));
        Err(failure())
    })
    .is_err());
    assert_eq!(actual(&flags), [false; 3]);
}
#[test]
fn disable_failure_recovers_every_removed_secondary_and_records_each_recovery_failure() {
    for failed in [Live, Long, Primary] {
        for recovery_failures in 0u8..4 {
            let flags = flags(7);
            let mut calls = Vec::new();
            let error = set_with(refs(&flags), false, |key, enabled| {
                calls.push((key, enabled));
                if (!enabled && key == failed)
                    || (enabled
                        && ((key == Live && recovery_failures & 1 != 0)
                            || (key == Long && recovery_failures & 2 != 0)))
                {
                    Err(failure())
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            let recovered_live = failed != Live;
            let recovered_long = failed == Primary;
            assert_eq!(
                actual(&flags),
                [
                    true,
                    !recovered_live || recovery_failures & 1 == 0,
                    !recovered_long || recovery_failures & 2 == 0,
                ]
            );
            assert_eq!(
                error.message().contains("Ctrl+2 recovery failed"),
                recovered_live && recovery_failures & 1 != 0
            );
            assert_eq!(
                error.message().contains("Ctrl+3 recovery failed"),
                recovered_long && recovery_failures & 2 != 0
            );
            let mut expected = vec![(Live, false)];
            if failed != Live {
                expected.push((Long, false));
            }
            if failed == Primary {
                expected.push((Primary, false));
            }
            if recovered_live {
                expected.push((Live, true));
            }
            if recovered_long {
                expected.push((Long, true));
            }
            assert_eq!(calls, expected);
        }
    }
}
#[test]
fn degraded_disable_recovers_only_previously_owned_keys() {
    for mask in [1, 3, 5] {
        let flags = flags(mask);
        let before = actual(&flags);
        assert!(set_with(refs(&flags), false, |key, enabled| {
            assert!(before[key as usize]);
            if key == Primary {
                Err(failure())
            } else {
                assert_eq!(enabled, !flags[key as usize].get());
                Ok(())
            }
        })
        .is_err());
        assert_eq!(actual(&flags), before);
    }
}
#[test]
fn destruction_attempts_every_owned_release_independently_and_keeps_failed_actual_flags() {
    for held in 0u8..8 {
        for failures in 0u8..8 {
            let flags = flags(held);
            let mut calls = Vec::new();
            let errors = cleanup_with(refs(&flags), |key| {
                calls.push(key);
                if failures & (1 << key as u8) != 0 {
                    Err(failure())
                } else {
                    Ok(())
                }
            });
            assert_eq!(
                calls,
                [Long, Live, Primary]
                    .into_iter()
                    .filter(|key| held & (1 << *key as u8) != 0)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual(&flags),
                [0, 1, 2].map(|bit| held & failures & (1 << bit) != 0)
            );
            assert_eq!(
                errors.iter().flatten().count(),
                (held & failures).count_ones() as usize
            );
        }
    }
}
