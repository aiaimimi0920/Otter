//! 仅对既有 linshi 合成会话验证 UI 归档提交，结束后恢复夹具基线供其余回归使用。
use super::*;
pub(in crate::desktop::application) unsafe fn verify(
    app: &mut Application,
    candidate: Candidate,
    stale: bool,
) -> Result<()> {
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
        return Err(Error::new(
            E_ABORT,
            "Archive smoke requires synthetic linshi session",
        ));
    }
    let original = app.restored_session.as_ref().unwrap().original.clone();
    let revision = candidate.ready().snapshot().revision();
    let directory = path.parent().unwrap().join(format!(
        "images-{}",
        crate::desktop::document_state::new_unit_id()?
    ));
    std::fs::create_dir(&directory).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let root = Arc::new(Root::open(&directory)?);
    let mut pending = candidate.start_archive(app, root, 123)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let staged = loop {
        if let Some(result) = pending.poll(app) {
            break result?;
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Archive UI worker timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    };
    if !app
        .restored_session
        .as_ref()
        .unwrap()
        .original
        .same_storage(&original)
    {
        return Err(Error::new(E_FAIL, "Archive staging changed UI baseline"));
    }
    if stale {
        let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
        app.documents[0].state.canvas.borrow_mut().normal_opacity =
            if opacity == 0.5 { 0.6 } else { 0.5 };
        let result = staged.commit(app);
        app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
        if !matches!(result, Err(ref error) if error.code() == E_ABORT)
            || !app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&original)
            || std::fs::read(&path).map_err(|e| Error::new(E_FAIL, e.to_string()))?
                != original.as_ref()
        {
            return Err(Error::new(E_FAIL, "Stale archive commit changed baseline"));
        }
        return Ok(());
    }
    let publication = staged.commit(app)?;
    let current = &app.restored_session.as_ref().unwrap().original;
    let decoded = crate::session_document::decode(current)
        .map_err(|_| Error::new(E_FAIL, "Archived UI session invalid"))?;
    let valid = decoded.revision == revision + 1
        && !current.same_storage(&original)
        && std::fs::read(&path).map_err(|e| Error::new(E_FAIL, e.to_string()))? == current.as_ref()
        && app.save_status() == save_state::Status::Unchanged
        && publication.retained_recovery.is_none();
    drop(decoded);
    // 仅回滚本回归的合成文件，避免后续无 Root 的恢复回归误读归档资源路径。
    std::fs::write(&path, original.as_ref()).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    app.restored_session.as_mut().unwrap().original = original;
    if !valid {
        return Err(Error::new(
            E_FAIL,
            "Archive UI commit did not publish consistent baseline",
        ));
    }
    Ok(())
}
