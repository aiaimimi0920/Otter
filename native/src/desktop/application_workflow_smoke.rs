//! 隐藏 HWND 保存链上的真实 worker 验证；不发送网络请求、不打开对话框。
use super::*;
pub(in crate::desktop::application) unsafe fn verify(app: &mut Application) -> Result<()> {
    // 保存回归允许使用已绑定 Art 而没有当前全局目录；工作流必须显式提供匹配目录。
    let catalog = app.documents.iter().find_map(|doc| {
        doc.state
            .selected_art
            .borrow()
            .as_ref()
            .map(|selected| selected.catalog.clone())
    });
    let previous = app.catalog.borrow_mut().loaded.clone();
    if catalog.is_some() {
        app.catalog.borrow_mut().loaded = catalog;
    }
    let result = verify_current(app);
    app.catalog.borrow_mut().loaded = previous;
    result
}
unsafe fn verify_current(app: &mut Application) -> Result<()> {
    let path = app.restored_session.as_mut().unwrap().saved_path.take();
    let unsaved_rejected = app.request_workflow_preparation(None).is_err();
    app.restored_session.as_mut().unwrap().saved_path = path;
    if !unsaved_rejected {
        return Err(Error::new(E_FAIL, "Workflow accepted uncommitted baseline"));
    }
    for mode in 0..6 {
        app.request_workflow_preparation(None)?;
        if app.request_workflow_preparation(None).is_ok() {
            return Err(Error::new(E_FAIL, "Workflow admitted duplicate worker"));
        }
        let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
        if mode == 1 {
            app.cancel_workflow_preparation();
        }
        if mode == 2 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity =
                if opacity == 0.5 { 0.6 } else { 0.5 };
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let result = loop {
            match app.take_workflow_preparation() {
                Ok(None) if std::time::Instant::now() < deadline => {
                    crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1))
                }
                Ok(None) => {
                    app.cancel_workflow_preparation();
                    break Err(Error::new(E_FAIL, "Workflow worker timeout"));
                }
                result => break result,
            }
        };
        app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
        match (mode, result) {
            (4 | 5, Ok(Some(candidate))) => archive::smoke::verify(app, candidate, mode == 5)?,
            (3, Ok(Some(candidate))) => {
                // 合成完成报告只测试本地消费凭证，不宣称网络同步已经成功。
                let completion = sync::completion::Completion::new(
                    candidate.receipt,
                    crate::desktop::session_candidate::WorkflowDelivery {
                        ready: candidate.ready,
                        report:
                            crate::session_document::workflow_snapshot::batch::dispatch::Report {
                                acknowledged: 1,
                                failure: None,
                                may_have_reached_remote: false,
                            },
                    },
                );
                app.restored_session.as_mut().unwrap().workflow_delivery = Some(Ok(completion));
                if app.request_workflow_preparation(None).is_ok() {
                    return Err(Error::new(
                        E_FAIL,
                        "Unconsumed workflow result was overwritten",
                    ));
                }
                let completion = app.take_workflow_delivery().unwrap()?;
                if app.take_workflow_delivery().is_some() || completion.current_ready(app).is_none()
                {
                    return Err(Error::new(
                        E_FAIL,
                        "Workflow completion was lost or consumed twice",
                    ));
                }
                let previous = app.restored_session.as_ref().unwrap().original.clone();
                app.restored_session.as_mut().unwrap().original = previous.to_vec().into();
                let stale = !completion.is_current(app) && completion.current_ready(app).is_none();
                app.restored_session.as_mut().unwrap().original = previous;
                if !stale || completion.report().acknowledged != 1 {
                    return Err(Error::new(
                        E_FAIL,
                        "Workflow completion lost receipt or remote report",
                    ));
                }
            }
            (0, Err(error)) => return Err(error),
            (0, Ok(Some(candidate))) if candidate.matches(app) => {
                if candidate.ready().snapshot().revision() == 0 {
                    return Err(Error::new(E_FAIL, "Workflow revision missing"));
                }
                let cancel = std::sync::atomic::AtomicBool::new(false);
                let budget = crate::byte_budget::Budget::new(1024 * 1024);
                let mut batch = candidate
                    .ready()
                    .batch(&cancel)
                    .map_err(|_| Error::new(E_FAIL, "Workflow batch preparation failed"))?;
                let request = batch
                    .current(&budget, 1024 * 1024, &cancel)
                    .map_err(|_| Error::new(E_FAIL, "Workflow batch encoding failed"))?
                    .ok_or_else(|| Error::new(E_FAIL, "Workflow batch missing global request"))?;
                if request.workflow_id() != "hook-live" || request.bytes().is_empty() {
                    return Err(Error::new(E_FAIL, "Workflow batch global request invalid"));
                }
                // 未发送，不能伪造 ACK；丢弃批次必须释放待发送缓冲。
                drop(batch);
                if budget.used() != 0 {
                    return Err(Error::new(E_FAIL, "Workflow batch leaked budget"));
                }
                app.documents[0].state.canvas.borrow_mut().normal_opacity =
                    if opacity == 0.5 { 0.6 } else { 0.5 };
                let stale = !candidate.matches(app);
                app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
                if !stale {
                    return Err(Error::new(
                        E_FAIL,
                        "Delivered workflow ignored later changes",
                    ));
                }
                let previous = app.restored_session.as_ref().unwrap().original.clone();
                app.restored_session.as_mut().unwrap().original = previous.to_vec().into();
                let source_rejected = !candidate.matches(app);
                app.restored_session.as_mut().unwrap().original = previous;
                let catalog = app.catalog.borrow_mut().loaded.take();
                let catalog_rejected = catalog.is_none() || !candidate.matches(app);
                app.catalog.borrow_mut().loaded = catalog;
                if !source_rejected || !catalog_rejected {
                    return Err(Error::new(
                        E_FAIL,
                        "Workflow ignored source or capability identity change",
                    ));
                }
                app.documents[0].state.canvas.borrow_mut().normal_opacity =
                    if opacity == 0.5 { 0.6 } else { 0.5 };
                let sync_rejected = app.request_workflow_sync(candidate).is_err();
                app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
                if !sync_rejected {
                    return Err(Error::new(E_FAIL, "Stale workflow started network worker"));
                }
            }
            (1 | 2, Err(_)) => (),
            _ => return Err(Error::new(E_FAIL, "Workflow receipt validation failed")),
        }
    }
    cycle::smoke::verify(app)?;
    ui::smoke::verify(app)?;
    println!("native-workflow-prepare-smoke: worker=true single-slot=true cancel=true stale=true receipt-retained=true network=false");
    Ok(())
}
