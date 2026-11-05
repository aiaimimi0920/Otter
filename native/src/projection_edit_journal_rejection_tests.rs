//! 明确拒绝与未知副作用分开；原请求、view与Checkpoint前持久标记均用独立磁盘oracle。
use super::*;
use crate::projection_edit::{fixture::*, Change, Code, Operation};
use serde_json::json;
fn apply(store: &Store) -> Ticket {
    store
        .begin(
            context("https://loom.example"),
            Route::Shared,
            Request::prepare(Operation::Apply {
                projection_id: PROJECTION.into(),
                session_id: SESSION.into(),
                op_id: "op-rejected".into(),
                base_revision: 2,
                mode_revision: 2,
                changes: vec![Change {
                    object_id: "shape-1".into(),
                    value: serde_json::Value::Null,
                }],
            })
            .unwrap(),
            Some(mode_document()),
            Default::default(),
            &AtomicBool::new(false),
        )
        .unwrap()
}
#[test]
fn rejected_apply_keeps_original_bytes_until_exact_checkpoint_path_and_reopen() {
    let directory = Directory::new();
    let store = directory.store();
    let ticket = apply(&store);
    let bytes = ticket.request().bytes().to_vec();
    let rejected = ticket
        .reject(Code::ObjectConflict, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(ticket.check(&AtomicBool::new(false)), Err(Error::Changed));
    assert_eq!(rejected.request().bytes(), bytes);
    assert_eq!(
        rejected.commit_after_checkpoint(mode_document(), &AtomicBool::new(false)),
        Err(Error::Changed)
    );
    let disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    assert_eq!(disk["entries"][0]["pending"]["rejected"], "object_conflict");
    assert_eq!(
        disk["entries"][0]["pending"]["request"]["opId"],
        "op-rejected"
    );
    drop((ticket, rejected, store));
    let store = directory.store();
    let mut pending = store.pending(&AtomicBool::new(false)).unwrap();
    assert_eq!(pending[0].rejection(), Some(Code::ObjectConflict));
    assert_eq!(pending[0].request().bytes(), bytes);
    pending
        .remove(0)
        .commit_rejected_after_checkpoint(mode_document(), &AtomicBool::new(false))
        .unwrap();
    assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
}
#[test]
fn unknown_pending_and_unrecognized_error_cannot_gain_rejection_retirement() {
    let directory = Directory::new();
    let store = directory.store();
    let ticket = apply(&store);
    let before = std::fs::read(directory.path()).unwrap();
    for code in [
        Code::Other,
        Code::Busy,
        Code::OperationReused,
        Code::BasisConflict,
        Code::SessionMismatch,
    ] {
        assert!(matches!(
            ticket.reject(code, &AtomicBool::new(false)),
            Err(Error::Format)
        ));
    }
    assert_eq!(
        ticket.commit_rejected_after_checkpoint(mode_document(), &AtomicBool::new(false)),
        Err(Error::Changed)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    assert!(matches!(
        ticket.reject(Code::ReadOnly, &AtomicBool::new(true)),
        Err(Error::Stopped)
    ));
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
}
#[test]
fn rejected_marker_on_initialize_or_unknown_code_and_changed_session_are_rejected() {
    let directory = Directory::new();
    let store = directory.store();
    let ticket = apply(&store)
        .reject(Code::ModeConflict, &AtomicBool::new(false))
        .unwrap();
    let before = std::fs::read(directory.path()).unwrap();
    let mut document = serde_json::to_value(mode_document()).unwrap();
    document["sessionId"] = json!("edit:33333333333333333333333333333333");
    let changed = Document::parse(&serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(
        ticket.commit_rejected_after_checkpoint(changed, &AtomicBool::new(false)),
        Err(Error::Changed)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    let mut disk: serde_json::Value = serde_json::from_slice(&before).unwrap();
    disk["entries"][0]["pending"]["rejected"] = json!("other");
    assert!(Ledger::decode(&serde_json::to_vec(&disk).unwrap()).is_err());
    disk["entries"][0]["pending"]["rejected"] = json!("read_only");
    disk["entries"][0]["pending"]["request"] = serde_json::to_value(initialize()).unwrap();
    assert!(Ledger::decode(&serde_json::to_vec(&disk).unwrap()).is_err());
}
#[test]
fn mode_and_checkpoint_rejection_survive_disk_reopen_without_retiring_unknown_requests() {
    for operation in [
        Operation::Mode {
            projection_id: PROJECTION.into(),
            session_id: SESSION.into(),
            op_id: "mode-rejected".into(),
            base_mode_revision: 2,
            mode: crate::projection_edit::Mode::OneWay,
        },
        Operation::Checkpoint {
            projection_id: PROJECTION.into(),
            session_id: SESSION.into(),
            expected_revision: 2,
        },
    ] {
        let directory = Directory::new();
        let store = directory.store();
        let ticket = store
            .begin(
                context("https://loom.example"),
                Route::Shared,
                Request::prepare(operation).unwrap(),
                Some(mode_document()),
                Default::default(),
                &AtomicBool::new(false),
            )
            .unwrap();
        let bytes = ticket.request().bytes().to_vec();
        let before = std::fs::read(directory.path()).unwrap();
        assert_eq!(
            ticket.commit_rejected_after_checkpoint(mode_document(), &AtomicBool::new(false)),
            Err(Error::Changed)
        );
        assert_eq!(std::fs::read(directory.path()).unwrap(), before);
        let rejected = ticket
            .reject(Code::RevisionConflict, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(rejected.request().bytes(), bytes);
        assert_eq!(
            rejected.commit_after_checkpoint(mode_document(), &AtomicBool::new(false)),
            Err(Error::Changed)
        );
        drop((ticket, rejected, store));
        let store = directory.store();
        let mut pending = store.pending(&AtomicBool::new(false)).unwrap();
        assert_eq!(pending[0].request().bytes(), bytes);
        assert_eq!(pending[0].rejection(), Some(Code::RevisionConflict));
        pending
            .remove(0)
            .commit_rejected_after_checkpoint(mode_document(), &AtomicBool::new(false))
            .unwrap();
        assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
    }
}
