//! 旧会话上下文输入的候选事务回归：注入输入结果，实际 worker/建窗/发布保持不变。
use super::*;
use std::time::{Duration, Instant};
const OLD: &[u8] = br#"{"stickers":[],"links":[]}"#;

pub(super) unsafe fn verify(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    app.naming.borrow_mut().sticker_save_pattern = "保持_{shortId}".into();
    let original = app.documents[0].window.0;
    for mode in [0, 1, 2] {
        app.request_restore_input(session_job::Input::Bytes(OLD.into()), None, None)?;
        let result = settle(&mut app, mode);
        if mode < 2 {
            if result.is_err() != (mode == 1) || !IsWindow(Some(original)).as_bool() {
                return Err(Error::new(
                    E_FAIL,
                    "Cancelled or stale coordinate input damaged workspace",
                ));
            }
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 1.0;
        } else {
            result?;
            if app.documents[0].state.naming.borrow().sticker_save_pattern != "保持_{shortId}" {
                return Err(Error::new(
                    E_FAIL,
                    "Workspace restore discarded runtime naming settings",
                ));
            }
            let space = app
                .restored_session
                .as_ref()
                .and_then(|record| record.space)
                .ok_or_else(|| Error::new(E_FAIL, "Chosen coordinate context missing"))?;
            if IsWindow(Some(original)).as_bool()
                || space.origin != (-1920, -200)
                || space.scale != 1.5
            {
                return Err(Error::new(
                    E_FAIL,
                    "Chosen legacy coordinates were not installed",
                ));
            }
        }
    }
    println!("native-session-context-smoke: cancel-preserved=true stale-rejected=true explicit-installed=true dialogs=false");
    Ok(())
}

unsafe fn settle(app: &mut Application, mode: u8) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.poll_restore_with(false, |app| {
            if mode == 0 {
                return Ok(None);
            }
            if mode == 1 {
                app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            }
            Ok(Some(crate::session_placement::Space {
                origin: (-1920, -200),
                scale: 1.5,
            }))
        })?;
        if Instant::now() >= deadline {
            app.cancel_restore();
            return Err(Error::new(E_FAIL, "Coordinate input fixture timed out"));
        }
        crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
    }
    Ok(())
}
