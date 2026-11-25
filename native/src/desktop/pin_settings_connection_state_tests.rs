//! Bind the source to the current Unit; same-named replacements cannot revive existing receipts or invalid sources. Tests write only owned temporary journals.
use super::*;
use crate::projection_delivery::{
    accept::tests::invitation,
    journal::{tests::Directory, Store},
};
use std::sync::Arc;
const ORIGIN: &str = "https://loom.example";
#[test]
fn presentation_labels_do_not_change_command_identity_but_eligibility_does() {
    let fields = |label, value, enabled, active| {
        vec![vec![(
            Field {
                id: SEND,
                label,
                icon: None,
                kind: Kind::Action {
                    value,
                    width: 80,
                    active,
                },
            },
            enabled,
        )]]
    };
    let source = fields("Send", "Source".into(), true, false);
    let translated = fields("投射", "译文".into(), true, false);
    assert!(projection_fields::same_commands(&source, &translated));
    assert!(!projection_fields::same_commands(
        &source,
        &fields("Send", "Source".into(), false, false)
    ));
    assert!(!projection_fields::same_commands(
        &source,
        &fields("Send", "Source".into(), true, true)
    ));
    assert!(!projection_fields::same_commands(&source, &[]));
}
#[test]
fn source_id_uses_current_unit_and_exit_requires_exact_saved_receipt() {
    unsafe {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let ticket = Arc::new(store.begin(&invitation(ORIGIN), "device-old").unwrap());
        let mut state = State::new(None, false).unwrap();
        state.authorization.borrow_mut().origin = Some(ORIGIN.into());
        state.authorization.borrow_mut().projection.recovery.saved = vec![ticket.clone()];
        state.authorization.borrow_mut().projection.recovery.entries = vec![ticket.clone()];
        assert!(
            source(&state).is_none(),
            "another Unit must not inherit the global first receipt"
        );
        state.unit_id = crate::workspace_id::UnitId::parse(ticket.unit()).unwrap();
        assert!(source(&state)
            .unwrap()
            .same(&ReceiveSource::Lan(ticket.clone())));
        assert_eq!(unlink(&state), Some(auth::projection::recovery::BASE));
        let fields = projection_fields::fields(&state, true).unwrap();
        let source_field = fields.iter().flatten().find(|(f, _)| f.id == 219).unwrap();
        assert!(
            matches!(&source_field.0.kind, Kind::Label { value, .. } if value == ticket.envelope().projection_id())
        );
        let snapshot = settings::Snapshot::capture(&state);
        state.authorization.borrow_mut().projection.recovery.entries =
            vec![Arc::new(ticket.retry())];
        assert!(!snapshot.same_inputs(&state));
        assert!(unlink(&state).is_none());
        assert!(
            source(&state)
                .unwrap()
                .same(&ReceiveSource::Lan(ticket.clone())),
            "pending cleanup must retain the saved source"
        );
        let fields = projection_fields::fields(&state, true).unwrap();
        assert!(
            !fields
                .iter()
                .flatten()
                .find(|(f, _)| f.id == UNLINK)
                .unwrap()
                .1
        );
        let snapshot = settings::Snapshot::capture(&state);
        state.authorization.borrow_mut().projection.recovery.saved = vec![Arc::new(ticket.retry())];
        assert!(!snapshot.same_inputs(&state));
        state.authorization.borrow_mut().origin = Some("https://other.example".into());
        assert!(source(&state).is_none());
        assert!(state.authorization.borrow().request.is_none());
    }
}

#[test]
fn account_receiver_exposes_source_and_queues_exact_binding_stop() {
    unsafe {
        let scope = serde_json::from_value(serde_json::json!("a".repeat(64))).unwrap();
        let (_, binding) =
            crate::projection_account::tests::scene_tests::fixture(scope, "receive-1");
        let mut state = State::new(None, false).unwrap();
        state.unit_id = crate::workspace_id::UnitId::parse(binding.unit()).unwrap();
        state.authorization.borrow_mut().origin = Some(binding.origin().into());
        state.authorization.borrow_mut().account.bindings = vec![binding.clone()];
        assert_eq!(
            source(&state).unwrap().id(),
            binding.envelope().projection_id()
        );
        state.pin_settings.borrow_mut().open = true;
        state.pin_settings.borrow_mut().connections.receive = true;
        let fields = refresh(&state, &mut state.pin_settings.borrow_mut()).unwrap();
        assert!(
            fields
                .iter()
                .flatten()
                .find(|(f, _)| f.id == COPY)
                .unwrap()
                .1
        );
        assert!(
            fields
                .iter()
                .flatten()
                .find(|(f, _)| f.id == UNLINK)
                .unwrap()
                .1
        );
        execute(HWND::default(), &state, UNLINK).unwrap();
        assert!(matches!(state.authorization.borrow_mut().request.take(),
            Some(auth::Request::AccountProjection(request))
                if matches!(*request, auth::account::Request::Stop(ref value) if value == &binding)));
        let snapshot = settings::Snapshot::capture(&state);
        state.authorization.borrow_mut().account.bindings = vec![binding.stop()];
        assert!(!snapshot.same_inputs(&state));
        assert!(source(&state).is_none());
        execute(HWND::default(), &state, UNLINK).unwrap();
        assert!(state.authorization.borrow().request.is_none());
    }
}
