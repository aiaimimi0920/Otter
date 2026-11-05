//! 写前、确认、同请求恢复与单在途；只创建linshi内独占合成目录，保留失败现场。
use super::*;
use crate::projection_edit::fixture::*;
#[path = "projection_edit_journal_storage_tests.rs"]
mod storage;
const ORIGIN: &str = "https://loom.example";
fn begin(store: &Store) -> Ticket {
    store
        .begin(
            context(ORIGIN),
            Route::Shared,
            initialize(),
            None,
            view(),
            &AtomicBool::new(false),
        )
        .unwrap()
}
#[test]
fn request_and_local_view_are_durable_before_ticket_and_reopen_uses_original_payload() {
    let directory = Directory::new();
    let store = directory.store();
    let ticket = begin(&store);
    let saved = std::fs::read(directory.path()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        value["entries"][0]["pending"]["request"]["operation"],
        "initialize"
    );
    assert_eq!(value["entries"][0]["pending"]["view"]["shape-1"]["x"], 3);
    assert!(!saved.starts_with(&[239, 187, 191]));
    let original = ticket.request().bytes().to_vec();
    let same = begin(&store);
    assert_eq!(same.request().bytes(), original);
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
    drop((ticket, same, store));
    let reopened = directory.store();
    let pending = reopened.pending(&AtomicBool::new(false)).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].request().bytes(), original);
    assert_eq!(pending[0].view(), &view());
    assert_eq!(pending[0].route(), Route::Shared);
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
}
#[test]
fn changed_pending_request_view_route_or_device_cannot_replace_ambiguous_side_effect() {
    let directory = Directory::new();
    let store = directory.store();
    let _ticket = begin(&store);
    let saved = std::fs::read(directory.path()).unwrap();
    let attach = Request::prepare(super::super::Operation::Attach {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
    })
    .unwrap();
    assert!(matches!(
        store.begin(
            context(ORIGIN),
            Route::Shared,
            attach,
            None,
            view(),
            &AtomicBool::new(false)
        ),
        Err(Error::Busy)
    ));
    assert!(matches!(
        store.begin(
            context(ORIGIN),
            Route::Offline,
            initialize(),
            None,
            view(),
            &AtomicBool::new(false)
        ),
        Err(Error::Busy)
    ));
    let other = Context::new(context(ORIGIN).scope(), "unit-1", ORIGIN, "device-2").unwrap();
    assert!(matches!(
        store.begin(
            other,
            Route::Shared,
            initialize(),
            None,
            view(),
            &AtomicBool::new(false)
        ),
        Err(Error::Changed)
    ));
    let mut changed = view();
    changed.get_mut("shape-1").unwrap()["x"] = 99.into();
    assert!(matches!(
        store.begin(
            context(ORIGIN),
            Route::Shared,
            initialize(),
            None,
            changed,
            &AtomicBool::new(false)
        ),
        Err(Error::Changed)
    ));
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
}
#[test]
fn profile_lock_single_network_permit_and_old_ticket_are_fail_closed() {
    let directory = Directory::new();
    let store = directory.store();
    assert!(matches!(
        Store::open(&directory.0, &AtomicBool::new(false)),
        Err(Error::Lock)
    ));
    let ticket = begin(&store);
    let duplicate = begin(&store);
    let permit = ticket.activate(&AtomicBool::new(false)).unwrap();
    assert!(matches!(
        duplicate.activate(&AtomicBool::new(false)),
        Err(Error::Busy)
    ));
    drop(permit);
    assert!(duplicate.activate(&AtomicBool::new(false)).is_ok());
    ticket
        .commit_after_checkpoint(document(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(ticket.check(&AtomicBool::new(false)), Err(Error::Changed));
    assert_eq!(
        duplicate.commit_after_checkpoint(document(), &AtomicBool::new(false)),
        Err(Error::Changed)
    );
    assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
}
#[test]
fn remote_success_is_not_checkpoint_and_next_operation_needs_exact_confirmed_previous() {
    let directory = Directory::new();
    let store = directory.store();
    let ticket = begin(&store);
    assert_eq!(
        store.observe(&context(ORIGIN), document(), &AtomicBool::new(false)),
        Err(Error::Busy)
    );
    let doc = ticket
        .request()
        .response(&serde_json::to_vec(&document()).unwrap(), None)
        .unwrap();
    assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
    ticket
        .commit_after_checkpoint(doc, &AtomicBool::new(false))
        .unwrap();
    let pending = store
        .begin(
            context(ORIGIN),
            Route::Shared,
            mode(),
            Some(document()),
            view(),
            &AtomicBool::new(false),
        )
        .unwrap();
    let saved = std::fs::read(directory.path()).unwrap();
    assert_eq!(
        pending.commit_after_checkpoint(document(), &AtomicBool::new(false)),
        Err(Error::Changed)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
    pending
        .commit_after_checkpoint(mode_document(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        store.observe(&context(ORIGIN), document(), &AtomicBool::new(false)),
        Err(Error::Changed)
    );
    assert!(store
        .begin(
            context(ORIGIN),
            Route::Shared,
            mode(),
            Some(document()),
            view(),
            &AtomicBool::new(false)
        )
        .is_err());
}
#[test]
fn cancel_before_disk_or_after_ticket_preserves_bytes_and_releases_network_permit() {
    let directory = Directory::new();
    let stopped = AtomicBool::new(true);
    assert!(matches!(
        Store::open(&directory.0, &stopped),
        Err(Error::Stopped)
    ));
    let store = directory.store();
    assert!(matches!(
        store.begin(
            context(ORIGIN),
            Route::Shared,
            initialize(),
            None,
            view(),
            &stopped
        ),
        Err(Error::Stopped)
    ));
    assert!(!directory.path().exists());
    let ticket = begin(&store);
    let saved = std::fs::read(directory.path()).unwrap();
    assert!(matches!(ticket.activate(&stopped), Err(Error::Stopped)));
    assert!(ticket.activate(&AtomicBool::new(false)).is_ok());
    assert_eq!(
        ticket.commit_after_checkpoint(document(), &stopped),
        Err(Error::Stopped)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
}
#[test]
fn stale_versions_read_requests_and_missing_previous_never_enter_write_ahead_log() {
    let directory = Directory::new();
    let store = directory.store();
    for request in [
        mode(),
        Request::prepare(super::super::Operation::Read {
            projection_id: PROJECTION.into(),
        })
        .unwrap(),
    ] {
        assert!(store
            .begin(
                context(ORIGIN),
                Route::Shared,
                request,
                None,
                view(),
                &AtomicBool::new(false)
            )
            .is_err());
    }
    let ticket = begin(&store);
    ticket
        .commit_after_checkpoint(mode_document(), &AtomicBool::new(false))
        .unwrap();
    assert!(store
        .begin(
            context(ORIGIN),
            Route::Shared,
            mode(),
            Some(mode_document()),
            view(),
            &AtomicBool::new(false)
        )
        .is_err());
    assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
}
#[test]
fn entry_limit_duplicate_keys_unknown_fields_and_context_are_checked() {
    let entry = Entry {
        context: context(ORIGIN),
        generation: 1,
        document: None,
        source: None,
        pending: Some(Pending {
            route: Route::Shared,
            request: initialize(),
            previous: None,
            view: view(),
            rejected: None,
        }),
    };
    let mut entries = Vec::new();
    for index in 0..MAX_ENTRIES {
        let mut e = entry.clone();
        e.context.unit = format!("unit-{index}");
        entries.push(e);
    }
    assert_eq!(Ledger::new(entries.clone()).validate(), Ok(()));
    let mut extra = entry.clone();
    extra.context.unit = "extra".into();
    entries.push(extra);
    assert_eq!(Ledger::new(entries).validate(), Err(Error::Limit));
    assert_eq!(
        Ledger::new(vec![entry.clone(), entry.clone()]).validate(),
        Err(Error::Format)
    );
    let mut value = serde_json::to_value(Ledger::new(vec![entry])).unwrap();
    value["entries"][0]["unexpected"] = true.into();
    assert!(Ledger::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    for origin in [
        "http://loom.example",
        "https://loom.example/extra",
        "https://user:secret@loom.example",
    ] {
        assert!(Context::new(context(ORIGIN).scope(), "unit", origin, "device").is_err());
    }
}
