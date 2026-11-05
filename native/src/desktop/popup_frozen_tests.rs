//! 全库可达性和页内身份映射；从不通过截断满足菜单树预算。
use super::*;
fn model(count: usize) -> LibraryMenu {
    let mut tree = Vec::new();
    let mut model = LibraryMenu::attach(&mut tree, Some(b"{}".to_vec().into()));
    model.data = Some([0, 1].map(|library| {
        Ok((0..count)
            .map(|i| index::Entry {
                id: format!("{library}-{i}"),
                label: format!("row {i}"),
            })
            .collect())
    }));
    model
}
#[test]
fn all_4096_entries_in_both_libraries_are_reachable_with_bounded_pages() {
    let mut model = model(4096);
    for library in 0..2 {
        let mut ids = std::collections::HashSet::new();
        let mut removed = std::collections::HashSet::new();
        for page in 0..64 {
            model.pages[library] = page;
            let rows = model.rows(library);
            assert_eq!(rows.len(), 67);
            for row in rows
                .into_iter()
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
    for command in NAV {
        assert!(model.take(command).is_none());
    }
    assert!(model.take(0).is_none());
}
#[test]
fn last_partial_page_never_reuses_an_old_slot_or_drops_its_last_entry() {
    let mut model = model(65);
    model.pages[0] = 1;
    let rows = model.rows(0);
    assert_eq!(rows.len(), 4);
    assert!(rows[1].enabled);
    assert!(!rows[3].enabled);
    assert_eq!(model.take(BASE).unwrap().entry_id, "0-64");
    assert!(model.take(BASE + 1).is_none());
}
#[test]
fn bad_library_is_isolated_from_valid_library_and_worker_is_reaped() {
    let source = serde_json::to_vec(&serde_json::json!({"recycleBin":42,"referenceLibrary":[{
        "entryId":"entry","sourceStickerId":"old","createdAt":"A&B","snapshot":{"id":"old","x":0,"y":0,"w":32,"h":32}
    }]})).unwrap().into();
    let mut job = index::Job::start(source).unwrap();
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
