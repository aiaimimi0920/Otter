//! Exercise one remote settings transaction across a real OS registration,
//! host replacement and disk conflict. No synthesized input or network access.
use super::*;
use crate::{
    desktop::{settings_startup, shortcut_registry as registry},
    loom_shortcut_protocol::Request,
    shortcut_policy::{Binding, Chord, Policy, GLOBAL},
};
use std::sync::{atomic::AtomicBool, Arc};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

fn policy(key: u8) -> Policy {
    Policy {
        bindings: GLOBAL
            .iter()
            .enumerate()
            .map(|(i, (id, _))| Binding {
                action: (*id).into(),
                chord: (i == 0).then_some(Chord { key, modifiers: 7 }),
            })
            .collect(),
        ..Policy::default()
    }
}

unsafe fn check_free(probe: HWND, key: u8) -> bool {
    let modifiers = MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT;
    if RegisterHotKey(Some(probe), 811, modifiers, u32::from(key)).is_err() {
        return false;
    }
    UnregisterHotKey(Some(probe), 811).unwrap();
    true
}

#[test]
fn conflicting_disk_write_restores_registration_on_transferred_host_then_offline_reloads() {
    unsafe {
        let _environment = environment();
        let mut app = app();
        let next = app
            .prepare_window(State::new(None, false).unwrap(), false)
            .unwrap();
        let probe = next.panel.0;
        let keys: Vec<_> = (0x70..=0x7a)
            .filter(|k| *k != 0x73 && check_free(probe, *k))
            .take(2)
            .collect();
        assert_eq!(
            keys.len(),
            2,
            "two free temporary function-key chords required"
        );
        let original = policy(keys[0]);
        let changed = policy(keys[1]);
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 2, "foreign": true,
            "shortcuts": { "revision": 0, "lastRequest": null, "policy": original }
        }))
        .unwrap();
        let file = Fixture::new(&bytes);
        let loaded = settings_startup::resolve_in_profile(Some(&file.0), true, None).unwrap();
        loaded.install(&app.documents[0].state).unwrap();
        app.install_startup_settings(loaded.source.unwrap())
            .unwrap();
        app.install_profile_target(Some(file.0.clone()));
        app.hotkey_enabled.set(true);
        registry::resume(app.documents[0].window.0, &app.documents[0].state).unwrap();
        assert!(!check_free(probe, keys[0]));
        let (_grant, lease) = super::art_issuer_tests::lease();
        let start = |app: &mut Application| {
            app.start_shortcut_save(
                "loom|device|gen".into(),
                Request {
                    request_id: "save".into(),
                    expected_revision: 0,
                    receiver_generation: "gen".into(),
                    expires_at: crate::desktop::surface_windows::now_ms().unwrap() + 15_000,
                    valid_for_ms: 15000,
                    policy: changed.clone(),
                },
                Instant::now() + Duration::from_secs(15),
                Arc::new(AtomicBool::new(false)),
                lease.clone(),
            )
            .expect("staged settings")
        };
        let settle = |mut save: super::super::super::shortcut_save::Save, app: &mut Application| {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                if let Some(r) = save.poll(app) {
                    break r;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        let external = br#"{"schemaVersion":2,"foreign":"external"}"#;
        fs::write(&file.0, external).unwrap();
        let save = start(&mut app);
        assert!(check_free(probe, keys[0]));
        assert!(!check_free(probe, keys[1]));
        super::super::super::restore_batch::transfer_hotkey(
            &app.documents,
            std::slice::from_ref(&next),
            |doc, attach| registry::transfer(doc.window.0, &doc.state, attach),
        )
        .unwrap();
        app.documents.push(next);
        let receipt = settle(save, &mut app);
        assert_eq!(
            (receipt.status, receipt.code, receipt.applied_revision),
            ("rejected", "settings_save_failed", None)
        );
        assert_eq!(fs::read(&file.0).unwrap(), external);
        assert_eq!(app.naming_document.shortcut_settings().unwrap().revision, 0);
        assert!(!registry::owns(&app.documents[0].state));
        assert!(registry::owns(&app.documents[1].state));
        assert!(!check_free(probe, keys[0]));
        assert!(check_free(probe, keys[1]));

        fs::write(&file.0, &bytes).unwrap();
        let retry = start(&mut app);
        assert_eq!(settle(retry, &mut app).applied_revision, Some(1));
        registry::close(app.documents[1].window.0, &app.documents[1].state);
        drop(app);
        let fresh = super::app();
        let saved = settings_startup::resolve_in_profile(Some(&file.0), true, None).unwrap();
        saved.install(&fresh.documents[0].state).unwrap();
        assert_eq!(*fresh.shortcut_policy.borrow(), changed);
        assert_eq!(
            saved
                .source
                .unwrap()
                .document()
                .shortcut_settings()
                .unwrap()
                .revision,
            1
        );
        let doc = &fresh.documents[0];
        fresh.hotkey_enabled.set(true);
        registry::resume(doc.window.0, &doc.state).unwrap();
        assert!(check_free(doc.panel.0, keys[0]));
        assert!(!check_free(doc.panel.0, keys[1]));
        registry::close(doc.window.0, &doc.state);
    }
}

#[test]
fn cancelled_shortcut_save_reawakens_exit_after_its_final_completion_wake() {
    unsafe {
        let _environment = environment();
        let mut app = app();
        let file = Fixture::new(br#"{"schemaVersion":2}"#);
        let loaded = settings_startup::resolve_in_profile(Some(&file.0), true, None).unwrap();
        app.install_startup_settings(loaded.source.unwrap())
            .unwrap();
        app.install_profile_target(Some(file.0.clone()));
        // Keep global registration disabled: this is an exit/worker lifetime test.
        registry::configure(app.documents[0].window.0, &app.documents[0].state, false).unwrap();
        let (_grant, lease) = super::art_issuer_tests::lease();
        let save = app
            .start_shortcut_save(
                "loom|device|gen".into(),
                Request {
                    request_id: "exit-save".into(),
                    expected_revision: 0,
                    receiver_generation: "gen".into(),
                    expires_at: crate::desktop::surface_windows::now_ms().unwrap() + 15_000,
                    valid_for_ms: 15000,
                    policy: policy(0x70),
                },
                Instant::now() + Duration::from_secs(15),
                Arc::new(AtomicBool::new(false)),
                lease,
            )
            .expect("staged settings");
        save.stop();
        save.test_wait_finished();
        // Recreate the loop after cancellation and worker notifications were consumed.
        app.shortcut_remote.test_pending(save);
        app.exit_requested.set(true);
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, WM_APP, 0xffff, PM_REMOVE).as_bool() {}
        app.execution_wake.acknowledge();
        assert!(!app.close_requested_with_notify(
            |_| panic!("unsettled shortcut save must block exit"),
            |_, e| panic!("{e}"),
        ));
        app.process_remote_shortcuts();
        assert!(!app.locale_blocks_exit());
        assert!(app.shortcut_remote.deadline().is_none());
        assert!(
            PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool(),
            "settling the last save must reschedule exit without user input"
        );
        app.execution_wake.acknowledge();
        assert!(app.close_requested_with_notify(
            |_| super::super::super::exit::Choice::Discard,
            |_, e| panic!("{e}"),
        ));
        assert!(app.exit_committed);
        assert!(!app.documents.iter().any(|d| d.is_alive()));
    }
}
