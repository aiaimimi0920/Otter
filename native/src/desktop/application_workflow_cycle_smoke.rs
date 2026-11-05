//! 隐藏会话周期回归：真实归档后撤销目录，确保新 revision 重准备失败且不发送网络。
use super::*;
pub(in crate::desktop::application) unsafe fn verify(app: &mut Application) -> Result<()> {
    let path = app
        .restored_session
        .as_ref()
        .unwrap()
        .saved_path
        .clone()
        .unwrap();
    let temporary = std::path::Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi")
        .canonicalize()
        .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    if !path
        .canonicalize()
        .map_err(|e| Error::new(E_FAIL, e.to_string()))?
        .starts_with(temporary)
    {
        return Err(Error::new(E_ABORT, "Cycle smoke requires linshi fixture"));
    }
    let original = app.restored_session.as_ref().unwrap().original.clone();
    let revision = crate::session_document::decode(&original)
        .map_err(|_| Error::new(E_FAIL, "Invalid cycle fixture"))?
        .revision;
    let directory = path.parent().unwrap().join(format!(
        "cycle-images-{}",
        crate::desktop::document_state::new_unit_id()?
    ));
    std::fs::create_dir(&directory).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let root = Arc::new(Root::open(&directory)?);
    app.request_workflow_cycle(root.clone(), 123)?;
    if app.request_workflow_cycle(root, 123).is_ok() {
        return Err(Error::new(E_FAIL, "Duplicate workflow cycle admitted"));
    }
    let mut removed_catalog = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let outcome = loop {
        app.process_workflow_cycle();
        if removed_catalog.is_none()
            && app
                .restored_session
                .as_ref()
                .unwrap()
                .workflow_cycle
                .as_ref()
                .is_some_and(|cycle| matches!(cycle.phase, Phase::PreparingSync))
        {
            removed_catalog = app.catalog.borrow_mut().loaded.take();
        }
        if let Some(outcome) = app.take_workflow_cycle() {
            break outcome;
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Workflow cycle timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    };
    app.catalog.borrow_mut().loaded = removed_catalog;
    let archived =
        crate::session_document::decode(&app.restored_session.as_ref().unwrap().original)
            .map_err(|_| Error::new(E_FAIL, "Archived cycle source invalid"))?
            .revision;
    let valid = outcome.sync.is_err()
        && !outcome.current
        && outcome.archive.is_some()
        && archived == revision + 1
        && app.take_workflow_cycle().is_none()
        && app
            .restored_session
            .as_ref()
            .unwrap()
            .workflow_sync
            .is_none();
    std::fs::write(&path, original.as_ref()).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    app.restored_session.as_mut().unwrap().original = original;
    if !valid {
        return Err(Error::new(
            E_FAIL,
            "Workflow cycle lost archive outcome or sent stale revision",
        ));
    }
    Ok(())
}
