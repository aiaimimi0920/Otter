//! 生产菜单分页选择 -> 退出模态后的单槽请求 -> 应用后台追加闭环。
use super::*;
use crate::session_library::Kind;
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    let old = app.documents[0].window.0;
    let png = crate::loom_base64::encode_png_data_url(
        super::super::smoke::image_fixture::PNG,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .map_err(|_| Error::new(E_FAIL, "Frozen menu fixture PNG"))?;
    let entries:Vec<_> = (0..65).map(|i|serde_json::json!({"entryId":format!("entry-{i}"),"sourceStickerId":format!("old-{i}"),"createdAt":"legacy",
        "snapshot":{"id":format!("old-{i}"),"src":png.as_str(),"x":0,"y":0,"w":32,"h":32}})).collect();
    app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],"recycleBin":entries,"referenceLibrary":entries})).unwrap().into();
    app.sync_frozen_menu();
    let source = app.restored_session.as_ref().unwrap().original.clone();
    for entry_id in ["entry-0", "entry-1"] {
        crate::desktop::frozen_library_ui::enqueue(
            old,
            &app.documents[0].state,
            crate::desktop::frozen_library_ui::Choice {
                remove: false,
                source: source.clone(),
                kind: Kind::RecycleBin,
                entry_id: entry_id.into(),
            },
            POINT { x: 0, y: 0 },
        )?;
    }
    if app
        .frozen_library
        .borrow()
        .request
        .as_ref()
        .unwrap()
        .choice
        .entry_id
        != "entry-0"
    {
        return Err(Error::new(E_FAIL, "Frozen requests were not coalesced"));
    }
    let tool = app.documents[0].state.canvas.borrow().tool;
    app.documents[0].state.canvas.borrow_mut().tool = if tool == crate::editor::Tool::Brush {
        crate::editor::Tool::Text
    } else {
        crate::editor::Tool::Brush
    };
    app.pending_windows(false)?;
    app.documents[0].state.canvas.borrow_mut().tool = tool;
    if app.restore_job.is_some()
        || app.documents.len() != 1
        || !app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&source)
    {
        return Err(Error::new(E_FAIL, "Frozen stale target was delivered"));
    }
    for library in 0..2 {
        let source = app.restored_session.as_ref().unwrap().original.clone();
        let choice = crate::desktop::popup_menu::smoke_frozen_selection(old, source, library)?;
        crate::desktop::frozen_library_ui::enqueue(
            old,
            &app.documents[0].state,
            choice,
            POINT { x: 100, y: 80 },
        )?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            app.pending_windows(false)?;
            if app.restore_job.is_none() && app.frozen_library.borrow().request.is_none() {
                break;
            }
            if std::time::Instant::now() >= deadline {
                app.cancel_restore();
                return Err(Error::new(E_FAIL, "Frozen menu delivery timeout"));
            }
            crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
        }
        if app.documents.len() != library + 2
            || !IsWindow(Some(old)).as_bool()
            || app
                .documents
                .last()
                .unwrap()
                .state
                .canvas
                .borrow()
                .image
                .is_none()
        {
            return Err(Error::new(
                E_FAIL,
                "Frozen menu failed to append selected image",
            ));
        }
    }
    let record = app.restored_session.as_ref().unwrap();
    let decoded = crate::session_document::decode(&record.original).unwrap();
    if decoded.library(Kind::RecycleBin).unwrap().entries().len() != 64
        || decoded
            .library(Kind::ReferenceLibrary)
            .unwrap()
            .entries()
            .len()
            != 65
    {
        return Err(Error::new(E_FAIL, "Frozen menu consumed wrong library"));
    }
    println!("native-frozen-menu-smoke: indexed=true paged=true selection=true queued=true worker=true appended=true libraries=true");
    Ok(())
}
