//! 纯键位/参数门禁；不读取或注入真实前台输入。
use super::*;

#[test]
fn private_arguments_are_exclusive_nonzero_decimal_and_bounded() {
    let args = |items: &[&str]| items.iter().map(OsString::from).collect::<Vec<_>>();
    assert_eq!(parse(&args(&[])).unwrap(), None);
    assert_eq!(parse(&args(&[FLAG, "42"])).unwrap(), Some(42));
    for items in [
        vec![FLAG],
        vec![FLAG, "0"],
        vec![FLAG, "-1"],
        vec![FLAG, "+1"],
        vec![FLAG, "4294967296"],
        vec![FLAG, " 42"],
        vec![FLAG, ""],
        vec![FLAG, "1", "--settings"],
        vec!["--settings", FLAG],
    ] {
        assert!(parse(&args(&items)).is_err(), "{items:?}");
    }
}

#[test]
fn startup_held_chords_do_not_arm_until_release_and_backup_is_an_edge() {
    let at = Instant::now();
    let mut gate = Gate::default();
    let held = Keys {
        escape: true,
        delete: true,
        backup: true,
    };
    for _ in 0..10 {
        assert!(!gate.observe(held, at));
    }
    assert!(!gate.observe(Keys::default(), at));
    assert!(gate.observe(held, at));
    assert!(!gate.observe(held, at));
    assert!(!gate.observe(Keys::default(), at));
    assert!(gate.observe(
        Keys {
            backup: true,
            ..Keys::default()
        },
        at
    ));
}

#[test]
fn watchdog_uses_the_same_three_full_release_chord_rule() {
    let at = Instant::now();
    let mut gate = Gate::default();
    assert!(!gate.observe(Keys::default(), at));
    for index in 0..3 {
        let now = at + Duration::from_millis(index * 100);
        assert_eq!(
            gate.observe(
                Keys {
                    escape: true,
                    delete: true,
                    backup: false
                },
                now
            ),
            index == 2
        );
        assert!(!gate.observe(
            Keys {
                escape: true,
                ..Keys::default()
            },
            now
        ));
        assert!(!gate.observe(
            Keys {
                escape: true,
                delete: true,
                backup: false
            },
            now
        ));
        assert!(!gate.observe(Keys::default(), now));
    }
}

#[path = "emergency_watchdog_runtime_tests.rs"]
mod runtime;
