//! 空会话保留可操作的非钉图宿主；原始会话和持久化标注清单仍为空。
use super::*;
use crate::desktop::{session_binding::Bound, session_candidate::VectorCandidate};
const EMPTY: &[u8] = br#"{"stickers":[],"links":[],"futureMetadata":{"retain":true}}"#;

pub(super) unsafe fn verify(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    let loaded = catalog()?;
    app.catalog.borrow_mut().loaded = Some(loaded.clone());
    let cancel = AtomicBool::new(false);
    for _ in 0..2 {
        let prior: Vec<_> = app.documents.iter().map(|doc| doc.window.0).collect();
        let candidate = crate::desktop::session_candidate::prepare(EMPTY.to_vec(), None, &cancel)?;
        let bound = Bound::prepare(
            VectorCandidate::prepare(candidate, &cancel)?,
            loaded.clone(),
            &cancel,
        )
        .map_err(|failure| failure.1)?;
        let prepared = bound
            .into_parts(&loaded, &cancel)
            .map_err(|_| Error::new(E_FAIL, "Empty fixture catalog changed"))?
            .prepare_states(&cancel)?;
        let batch = app.prepare_restored_batch(prepared, &cancel, |_| {
            Err(Error::new(
                E_FAIL,
                "Empty session unexpectedly requested saved geometry",
            ))
        })?;
        if batch.window_count() != 0 {
            return Err(Error::new(
                E_FAIL,
                "Empty preparation created a persisted document",
            ));
        }
        app.publish_restored_batch(batch, false)?;
        let record = app
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "Empty record missing"))?;
        let host = &app.documents[0];
        if app.documents.len() != 1
            || app.workspace.units().len() != 1
            || record.original.as_slice() != EMPTY
            || !record.annotation_ids.is_empty()
            || record.transient_host.as_ref() != Some(&host.state.unit_id)
            || !app.workspace.contains(&host.state.unit_id)
            || host.state.canvas.borrow().pinned
            || host.state.canvas.borrow().image.is_some()
            || IsWindowVisible(host.window.0).as_bool()
            || app.capture_windows.borrow().len() != 2
            || prior.iter().any(|hwnd| IsWindow(Some(*hwnd)).as_bool())
        {
            return Err(Error::new(
                E_FAIL,
                "Empty restoration transient host invariant failed",
            ));
        }
        app.pending_windows(false)?;
        app.add_window(false)?;
        if app.documents.len() != 2 || app.workspace.units().len() != 2 {
            return Err(Error::new(
                E_FAIL,
                "Empty restored workspace cannot create documents",
            ));
        }
    }
    let remaining: Vec<_> = app.documents.iter().map(|doc| doc.window.0).collect();
    drop(app);
    if remaining.iter().any(|hwnd| IsWindow(Some(*hwnd)).as_bool()) {
        return Err(Error::new(E_FAIL, "Empty restoration leaked host windows"));
    }
    Ok(())
}
