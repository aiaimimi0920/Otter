//! 复用生产保存/自动保存路径，缓存偏好也进入快照与过期判断；所有文件为合成夹具。
use super::*;
fn policy() -> crate::cache_policy::Policy {
    crate::cache_policy::Policy {
        temp_cache_max_bytes: 0,
        temp_cache_retention_days: 30,
    }
}
#[test]
fn manual_save_persists_cache_fields_and_preserves_unowned_cache_extensions() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        fs::write(files.source(), br#"{"schemaVersion":1,"cache":{"recycleBinMaxEntries":77,"future":{"keep":true}},"fileNaming":{}}"#).unwrap();
        queue(&app);
        app.pending_naming_import_with(|_, _| Ok(Some(files.source())))
            .unwrap();
        finish(&mut app).unwrap();
        app.cache_policy.set(policy());
        begin(&mut app).unwrap();
        finish_save(&mut app).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(files.source()).unwrap()).unwrap();
        assert_eq!(value["cache"]["tempCacheMaxBytes"], 0);
        assert_eq!(value["cache"]["tempCacheRetentionDays"], 30);
        assert_eq!(value["cache"]["recycleBinMaxEntries"], 77);
        assert_eq!(value["cache"]["future"]["keep"], true);
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(app.naming_document.cache_policy().unwrap(), policy());
    }
}
#[test]
fn cache_only_changes_autosave_and_participate_in_exit_wait() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        toggle(&app);
        let naming = app.naming.borrow().clone();
        app.cache_policy.set(policy());
        assert!(app.naming_autosave_blocks_exit());
        settle(&mut app).unwrap();
        assert!(!app.naming_autosave_blocks_exit());
        assert_eq!(*app.naming.borrow(), naming);
        assert_eq!(app.naming_document.cache_policy().unwrap(), policy());
        let bytes = fs::read(files.source()).unwrap();
        settle(&mut app).unwrap();
        assert_eq!(fs::read(files.source()).unwrap(), bytes);
    }
}
#[test]
fn cache_change_during_worker_or_path_selection_rejects_stale_publication() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        let old = fs::read(files.source()).unwrap();
        begin(&mut app).unwrap();
        app.cache_policy.set(policy());
        assert!(finish_save(&mut app).is_err());
        assert_eq!(fs::read(files.source()).unwrap(), old);
        queue_save(&app);
        assert!(app
            .pending_naming_save_with(|_, state| {
                state.cache_policy.set(Default::default());
                Ok(Some(files.target()))
            })
            .is_err());
        assert!(!files.target().exists());
        assert!(app.naming_save_job.is_none());
    }
}
