//! 仅消费同组新建的 linshi 文件，验证 Application 的后台替换及最终凭证保护。
use super::*;
use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
pub(super) unsafe fn verify(app: &mut Application, path: &Path) -> Result<()> {
    save_state::verify(app)?;
    if app.save_requires_destination(false) || !app.save_requires_destination(true) {
        return Err(Error::new(
            E_FAIL,
            "Committed workspace save routing mismatch",
        ));
    }
    let original = app.restored_session.as_ref().unwrap().original.clone();
    app.request_save_existing()?;
    let prepared = settle(app)?;
    if !matches!(prepared.staged, Some(save_disk::Disk::Replace(_))) {
        return Err(Error::new(
            E_FAIL,
            "Existing save did not prepare replacement",
        ));
    }
    let expected = prepared.candidate.bytes.clone();
    let published = prepared.commit_ready(app, &AtomicBool::new(false))?;
    if published.retained_recovery.is_some()
        || published.destination != path
        || std::fs::read(path).map_err(io_error)? != expected.as_ref()
        || app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&original)
    {
        return Err(Error::new(
            E_FAIL,
            "Application replacement did not publish new baseline",
        ));
    }
    let baseline = app.restored_session.as_ref().unwrap().original.clone();
    save_state::verify(app)?;
    app.request_save_existing()?;
    let prepared = settle(app)?;
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.documents[0].state.canvas.borrow_mut().normal_opacity =
        if opacity == 0.5 { 0.6 } else { 0.5 };
    let result = prepared.commit_ready(app, &AtomicBool::new(false));
    let remains_changed = app.save_status() == save_state::Status::Changed;
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    if !remains_changed
        || !matches!(result, Err(ref error) if error.code() == E_ABORT)
        || !app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&baseline)
        || std::fs::read(path).map_err(io_error)? != baseline.as_ref()
    {
        return Err(Error::new(
            E_FAIL,
            "Late stale replacement changed file or baseline",
        ));
    }
    // 失败准备结果必须释放来源 DELETE 句柄，而不是留到应用退出。
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(io_error)?;
    println!("native-save-replace-smoke: worker=true baseline-updated=true late-stale-preserved=true handles-released=true");
    exit_save_smoke::verify(app, path)?;
    open_session::smoke::verify(app.instance, path)?;
    Ok(())
}
unsafe fn settle(app: &mut Application) -> Result<save_publish::Prepared> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match app.take_save_candidate()? {
            Some(prepared) => return Ok(prepared),
            None if Instant::now() < deadline => {
                crate::desktop::smoke_timing::sleep(Duration::from_millis(1))
            }
            None => {
                app.cancel_save_candidate();
                app.save_job.take();
                return Err(Error::new(E_FAIL, "Replacement worker timed out"));
            }
        }
    }
}
fn io_error(error: std::io::Error) -> Error {
    Error::new(E_FAIL, error.to_string())
}
