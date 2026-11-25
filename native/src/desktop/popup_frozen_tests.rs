//! Full-library batchwise reachability and stable identity; appending neither reuses old slots nor truncates libraries to meet static-tree budgets.
use super::*;
fn model(count: usize) -> LibraryMenu {
    let mut tree = Vec::new();
    let mut model = LibraryMenu::attach(&mut tree, Some(b"{}".to_vec().into()), Workers::default());
    model.data = Some([0, 1].map(|library| {
        Ok((0..count)
            .map(|i| index::Entry {
                id: format!("{library}-{i}"),
                label: format!("row {i}"),
                source_unit: format!("source-{i}"),
            })
            .collect())
    }));
    model
}
#[test]
fn all_4096_entries_in_both_libraries_keep_identity_when_appended() {
    let mut model = model(4096);
    for library in 0..2 {
        let mut ids = std::collections::HashSet::new();
        let mut removed = std::collections::HashSet::new();
        for batch in 0..64 {
            model.loaded[library] = (batch + 1) * BATCH;
            let rows = model.rows(library);
            assert_eq!(rows.len(), model.loaded[library]);
            for (slot, row) in rows.iter().enumerate().skip(batch * BATCH) {
                let choice = model.take(row.delete.expect("Card delete button")).unwrap();
                assert!(choice.remove);
                assert_eq!(choice.entry_id, format!("{library}-{slot}"));
                assert!(matches!(
                    (library, choice.kind),
                    (0, Kind::RecycleBin) | (1, Kind::ReferenceLibrary)
                ));
            }
            for row in rows
                .into_iter()
                .skip(batch * BATCH)
                .flat_map(|row| row.children.unwrap_or_default())
            {
                if let Some(choice) = model.take(row.command) {
                    assert!(if choice.remove {
                        removed.insert(choice.entry_id)
                    } else {
                        ids.insert(choice.entry_id)
                    });
                    assert!(choice.source.same_storage(model.source.as_ref().unwrap()));
                }
            }
        }
        assert_eq!(ids.len(), 4096);
        assert_eq!(removed, ids);
        assert!(ids.contains(&format!("{library}-4095")));
    }
    assert!(model.take(0).is_none());
    assert!(model.take(REMOVE + CAPACITY * 2).is_none());
    assert!(model.take(usize::MAX - 25).is_none());
}
#[test]
fn trailing_batch_never_reuses_an_old_slot_or_exposes_an_unloaded_entry() {
    let mut model = model(65);
    assert_eq!(model.rows(0).len(), 64);
    assert!(model.take(BASE + 64).is_none());
    assert!(model.take(REMOVE + 64).is_none());
    model.loaded[0] = 65;
    let rows = model.rows(0);
    assert_eq!(rows.len(), 65);
    assert_eq!(model.take(BASE).unwrap().entry_id, "0-0");
    assert_eq!(model.take(BASE + 64).unwrap().entry_id, "0-64");
    assert_eq!(model.take(BASE + CAPACITY).unwrap().entry_id, "1-0");
    assert!(model.take(BASE + 65).is_none());
}
#[test]
fn bad_library_is_isolated_from_valid_library_and_worker_is_reaped() {
    let source = serde_json::to_vec(&serde_json::json!({"recycleBin":42,"referenceLibrary":[{
        "entryId":"entry","sourceStickerId":"old","createdAt":"A&B","snapshot":{"id":"old","x":0,"y":0,"w":32,"h":32}
    }]})).unwrap().into();
    let slot = worker::Slot::default();
    let mut job = index::Job::start(&slot, source, None).unwrap().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let result = loop {
        if let Some(result) = job.poll() {
            break result;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(result[0].is_err());
    assert_eq!(result[1].as_ref().unwrap()[0].id, "entry");
    assert!(result[1].as_ref().unwrap()[0].label.contains("A&&B"));
    assert!(job.poll().is_none());
}
#[test]
fn reference_command_uses_source_identity_and_never_enables_an_unknown_or_locked_state() {
    use super::super::super::reference_ui::{REMOVE, SET};
    let mut model = model(1);
    model.reference = Some((5, "source-0".into(), true));
    assert_eq!(
        model.reference_action(),
        (
            REMOVE,
            crate::ui_text::text("library-remove-favorite"),
            true
        )
    );
    model.reference.as_mut().unwrap().1 = "unreferenced".into();
    assert_eq!(
        model.reference_action(),
        (SET, crate::ui_text::text("library-add-favorite"), true)
    );
    model.reference.as_mut().unwrap().2 = false;
    assert!(!model.reference_action().2);
    model.data = Some([Ok(vec![]), Err("unavailable".into())]);
    model.reference.as_mut().unwrap().2 = true;
    assert_eq!(
        model.reference_action(),
        (SET, crate::ui_text::text("library-add-favorite"), false)
    );
    model.data = None;
    assert_eq!(
        model.reference_action(),
        (SET, crate::ui_text::text("library-add-favorite"), false)
    );
}

#[test]
fn real_menu_loads_trailing_entry_without_paging_and_selects_both_actions() {
    use windows::{
        core::w,
        Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
    };
    struct Owner(HWND);
    impl Drop for Owner {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    let owner = Owner(unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("frozen test"),
            WS_OVERLAPPED,
            0,
            0,
            100,
            100,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let entries: Vec<_> = (0..65).map(|n| serde_json::json!({"entryId":format!("entry-{n}"),"sourceStickerId":format!("source-{n}"),"createdAt":"", "snapshot":{"id":format!("source-{n}"),"x":0,"y":0,"w":2,"h":1}})).collect();
    let source: Source =
        serde_json::to_vec(&serde_json::json!({"recycleBin":entries,"referenceLibrary":entries}))
            .unwrap()
            .into();
    for library in 0..2 {
        for remove in [false, true] {
            let choice = smoke::select_action(owner.0, source.clone(), library, remove).unwrap();
            assert_eq!(choice.entry_id, "entry-64");
            assert_eq!(choice.remove, remove);
            assert!(choice.source.same_storage(&source));
        }
    }
}
