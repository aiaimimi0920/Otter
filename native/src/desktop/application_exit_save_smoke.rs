//! 正式保存 UI coordinator 的隐藏集成：只写调用方创建的 linshi 合成会话。
use super::*;
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) unsafe fn verify(app: &mut Application, path: &Path) -> Result<()> {
    let count = app.documents.len();
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.documents[0].state.canvas.borrow_mut().normal_opacity = altered(opacity);
    app.exit_requested.set(true);
    if app.close_requested_with(|_| exit::Choice::Save) || !app.exit_after_save {
        return Err(Error::new(E_FAIL, "Changed exit did not request save"));
    }
    let mut notices = Vec::new();
    app.pending_save_ui_with(|_, _, _, warning| notices.push(warning));
    if app.save_job.is_none() || app.save_origin.is_none() {
        return Err(Error::new(
            E_FAIL,
            "Exit save did not start UI-owned worker",
        ));
    }
    settle(app, &mut notices)?;
    if !app.exit_requested.replace(false)
        || app.exit_after_save
        || app.save_status() != save_state::Status::Unchanged
        || !notices.is_empty()
        || app.documents.len() != count
    {
        return Err(Error::new(
            E_FAIL,
            "Committed exit save did not defer clean exit",
        ));
    }
    let baseline = app.restored_session.as_ref().unwrap().original.clone();
    if read(path)? != baseline.as_ref() {
        return Err(Error::new(E_FAIL, "Exit save baseline differs from disk"));
    }

    // 已在保存时请求退出也挂靠同一任务；之后发生编辑必须拒绝提交且不退出。
    app.request_save_existing()?;
    app.save_origin = Some(app.documents[0].state.unit_id.clone());
    app.exit_requested.set(true);
    if app.close_requested_with(|_| panic!("Busy save must not show exit dialog"))
        || !app.exit_after_save
    {
        return Err(Error::new(E_FAIL, "Busy save did not retain exit intent"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    settle(app, &mut notices)?;
    if app.exit_requested.get()
        || app.exit_after_save
        || notices != [true]
        || app.documents.len() != count
        || read(path)? != baseline.as_ref()
    {
        return Err(Error::new(E_FAIL, "Stale exit save lost document or file"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = altered(opacity);

    // 完成时恰有新的保存请求：取消旧自动退出，保留后续保存，不让请求越过退出。
    app.request_save_existing()?;
    app.save_origin = Some(app.documents[0].state.unit_id.clone());
    app.exit_after_save = true;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.save_job.as_ref().unwrap().1.is_ready() {
        if Instant::now() >= deadline {
            app.cancel_save_candidate();
            return Err(Error::new(E_FAIL, "Queued exit save worker timed out"));
        }
        crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
    }
    app.documents[0].state.save_existing_requested.set(true);
    notices.clear();
    settle(app, &mut notices)?;
    if app.exit_requested.get()
        || app.exit_after_save
        || notices != [false, false]
        || app.documents.len() != count
        || app.save_status() != save_state::Status::Unchanged
        || read(path)? != app.restored_session.as_ref().unwrap().original.as_ref()
    {
        return Err(Error::new(
            E_FAIL,
            "Queued save was bypassed by automatic exit",
        ));
    }
    println!("native-exit-save-ui-smoke: committed=true deferred=true stale-preserved=true queued-preserved=true dialogs=false");
    Ok(())
}

unsafe fn settle(app: &mut Application, notices: &mut Vec<bool>) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.save_job.is_some() {
        app.pending_save_ui_with(|_, _, _, warning| notices.push(warning));
        if Instant::now() >= deadline {
            app.cancel_save_candidate();
            return Err(Error::new(E_FAIL, "Exit save UI coordinator timed out"));
        }
        crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
    }
    Ok(())
}
fn altered(opacity: f32) -> f32 {
    if opacity == 0.5 {
        0.6
    } else {
        0.5
    }
}
fn read(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| Error::new(E_FAIL, error.to_string()))
}
