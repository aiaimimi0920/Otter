//! 真实 HWND 请求和保存调度，注入目录选择/通知出口，不显示对话框或发送网络。
use super::*;
pub(in crate::desktop::application) unsafe fn verify(app: &mut Application) -> Result<()> {
    let directory =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-workflow-ui-{}",
            crate::desktop::document_state::new_unit_id()?
        ));
    std::fs::create_dir(&directory).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    verify_dialog_catalog_change(app, &directory)?;
    let mut root = Some(Root::open(&directory)?);
    let mut notices = Vec::new();
    request(app.documents[0].window.0, &app.documents[0].state, ENABLE)?;
    app.pending_workflow_ui_with(
        |_| Ok(root.take()),
        |_, message, warning| notices.push((message.to_owned(), warning)),
    );
    if root.is_some()
        || app
            .restored_session
            .as_ref()
            .unwrap()
            .workflow_cycle
            .is_none()
    {
        return Err(Error::new(
            E_FAIL,
            "Workflow menu did not select root and start cycle",
        ));
    }
    stop_before_network(app, &mut notices)?;
    let source = app.restored_session.as_ref().unwrap().original.clone();
    app.queue_saved_workflow();
    app.queue_saved_workflow();
    if !app
        .restored_session
        .as_ref()
        .unwrap()
        .workflow_settings
        .as_ref()
        .unwrap()
        .pending
        .as_ref()
        .is_some_and(|p| p.same_storage(&source))
    {
        return Err(Error::new(E_FAIL, "Workflow save intents did not coalesce"));
    }
    app.pending_workflow_ui_with(
        |_| panic!("Automatic save must reuse admitted root"),
        |_, message, warning| notices.push((message.to_owned(), warning)),
    );
    if app
        .restored_session
        .as_ref()
        .unwrap()
        .workflow_cycle
        .is_none()
    {
        return Err(Error::new(E_FAIL, "Saved workflow did not start cycle"));
    }
    stop_before_network(app, &mut notices)?;
    request(app.documents[0].window.0, &app.documents[0].state, DISABLE)?;
    app.pending_workflow_ui_with(
        |_| panic!("Disable must not choose root"),
        |_, message, warning| notices.push((message.to_owned(), warning)),
    );
    app.queue_saved_workflow();
    if app
        .restored_session
        .as_ref()
        .unwrap()
        .workflow_settings
        .as_ref()
        .unwrap()
        .pending
        .is_some()
        || notices.iter().filter(|(_, warning)| *warning).count() != 2
    {
        return Err(Error::new(
            E_FAIL,
            "Workflow disable or completion notifications failed",
        ));
    }
    app.restored_session.as_mut().unwrap().workflow_settings = None;
    std::fs::remove_dir(directory).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    Ok(())
}
unsafe fn verify_dialog_catalog_change(
    app: &mut Application,
    directory: &std::path::Path,
) -> Result<()> {
    let catalog = app.catalog.clone();
    let mut previous = None;
    let mut rejected = false;
    request(app.documents[0].window.0, &app.documents[0].state, ENABLE)?;
    app.pending_workflow_ui_with(
        |_| {
            previous = catalog.borrow_mut().loaded.take();
            Root::open(directory).map(Some)
        },
        |_, message, warning| {
            rejected = warning && message.contains("选择目录期间");
        },
    );
    catalog.borrow_mut().loaded = previous;
    let record = app.restored_session.as_ref().unwrap();
    if !rejected || record.workflow_cycle.is_some() || record.workflow_settings.is_some() {
        return Err(Error::new(
            E_FAIL,
            "Changed catalog during folder selection must not receive sync authority",
        ));
    }
    Ok(())
}
unsafe fn stop_before_network(
    app: &mut Application,
    notices: &mut Vec<(String, bool)>,
) -> Result<()> {
    let catalog = app.catalog.borrow_mut().loaded.take();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app.process_workflow_cycle();
        app.pending_workflow_ui_with(
            |_| panic!("No new directory request"),
            |_, message, warning| notices.push((message.to_owned(), warning)),
        );
        if app
            .restored_session
            .as_ref()
            .unwrap()
            .workflow_cycle
            .is_none()
        {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Workflow UI cancellation timed out"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    app.catalog.borrow_mut().loaded = catalog;
    if app
        .restored_session
        .as_ref()
        .unwrap()
        .workflow_sync
        .is_some()
    {
        return Err(Error::new(
            E_FAIL,
            "Workflow UI unexpectedly started network",
        ));
    }
    Ok(())
}
