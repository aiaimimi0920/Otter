//! 打开工作区菜单到后台发布；仅使用保存回归已创建的 linshi 文件，替换选择框出口。
use super::*;
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(in crate::desktop::application) unsafe fn verify(
    instance: HINSTANCE,
    path: &Path,
) -> Result<()> {
    // 保存事务返回 canonical/verbatim 路径；模拟文件框返回普通磁盘路径，不放宽读取准入。
    let display = path
        .to_str()
        .ok_or_else(|| Error::new(E_FAIL, "Invalid fixture path"))?;
    let selected = PathBuf::from(display.strip_prefix(r"\\?\").unwrap_or(display));
    let path = selected.as_path();
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    let hwnd = app.documents[0].window.0;
    let state = &app.documents[0].state;
    state.group_locked.set(true);
    for command in [OPEN_SESSION, OPEN_SESSION_ASSETS] {
        super::super::super::context_menu::execute(hwnd, state, command)?;
    }
    state.group_locked.set(false);
    let flag = state.file_dialog.clone();
    let modal = super::super::super::file_dialog::Guard::begin(&flag).unwrap();
    app.pending_open_session_with(|_, _, _| panic!("Modal must defer chooser"))?;
    if !app.documents[0].state.open_session_requested.get() {
        return Err(Error::new(E_FAIL, "Modal open request was lost"));
    }
    drop(modal);
    let mut choices = 0;
    app.pending_open_session_with(|_, _, assets| {
        if !assets {
            return Err(Error::new(
                E_FAIL,
                "Explicit asset selection request was lost",
            ));
        }
        choices += 1;
        Ok(None)
    })?;
    app.pending_open_session_with(|_, _, _| panic!("Coalesced chooser delivered twice"))?;
    if choices != 1 || app.restore_job.is_some() || !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_FAIL, "Cancelled chooser modified workspace"));
    }
    request(hwnd, &app.documents[0].state)?;
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    let stale = app.pending_open_session_with(|app, _, _| {
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
        Ok(Some(path.to_path_buf().into()))
    });
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    if stale.is_ok() || app.restore_job.is_some() || !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_FAIL, "Stale chooser replaced workspace"));
    }
    request(hwnd, &app.documents[0].state)?;
    app.pending_open_session_with(|_, _, _| {
        Ok(Some(path.with_extension("missing-open-test").into()))
    })?;
    if settle(&mut app).is_ok() || !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_FAIL, "Failed open destroyed source window"));
    }
    request(hwnd, &app.documents[0].state)?;
    app.pending_open_session_with(|_, _, _| Ok(Some(path.to_path_buf().into())))?;
    settle(&mut app)?;
    if IsWindow(Some(hwnd)).as_bool()
        || app.save_status() != save_state::Status::Unchanged
        || app
            .restored_session
            .as_ref()
            .and_then(|record| record.saved_path.as_deref())
            != Some(path)
        || app
            .documents
            .iter()
            .any(|doc| IsWindowVisible(doc.window.0).as_bool())
    {
        return Err(Error::new(
            E_FAIL,
            "Open workspace UI did not publish hidden saved session",
        ));
    }
    println!("native-open-session-ui-smoke: menu=true coalesced=true modal=true cancel=true stale=true failed-preserved=true restored=true dialogs=false");
    verify_save_first(&mut app, path)?;
    Ok(())
}

unsafe fn verify_save_first(app: &mut Application, path: &Path) -> Result<()> {
    for fail in [true, false] {
        let hwnd = app.documents[0].window.0;
        let before = std::fs::read(path).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
        request(hwnd, &app.documents[0].state)?;
        app.pending_open_session_with(|_, _, _| {
            Ok(Some(Selection {
                path: path.into(),
                save_first: true,
                root: Some(crate::desktop::session_asset_root::Root::open(
                    path.parent().unwrap(),
                )?),
            }))
        })?;
        let mut warnings = Vec::new();
        app.pending_save_ui_with(|_, _, _, warning| warnings.push(warning));
        if app.save_job.is_none() || app.open_after_save.is_none() || app.restore_job.is_some() {
            return Err(Error::new(
                E_FAIL,
                "Save before open did not defer restoration",
            ));
        }
        if fail {
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.7;
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.save_job.is_some() {
            app.pending_save_ui_with(|_, _, _, warning| warnings.push(warning));
            if Instant::now() >= deadline {
                app.cancel_save_candidate();
                return Err(Error::new(E_FAIL, "Save before open worker timed out"));
            }
            crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
        }
        if app.open_after_save.is_some() || app.exit_requested.get() {
            return Err(Error::new(
                E_FAIL,
                "Open continuation retained stale intent",
            ));
        }
        if fail {
            if app.restore_job.is_some()
                || warnings != [true]
                || !IsWindow(Some(hwnd)).as_bool()
                || std::fs::read(path).map_err(|e| Error::new(E_FAIL, e.to_string()))? != before
            {
                return Err(Error::new(
                    E_FAIL,
                    "Failed save continued opening or changed file",
                ));
            }
        } else {
            if app.restore_job.is_none() || !warnings.is_empty() {
                return Err(Error::new(E_FAIL, "Successful save did not start opening"));
            }
            settle(app)?;
            if IsWindow(Some(hwnd)).as_bool() || app.save_status() != save_state::Status::Unchanged
            {
                return Err(Error::new(
                    E_FAIL,
                    "Save then open did not replace clean workspace",
                ));
            }
        }
    }
    app.open_after_save = Some(Target {
        path: path.into(),
        root: None,
    });
    app.documents[0].state.save_existing_requested.set(true);
    let continued = app.finish_open_save(true)?;
    app.documents[0].state.save_existing_requested.set(false);
    if continued || app.open_after_save.is_some() || app.restore_job.is_some() {
        return Err(Error::new(
            E_FAIL,
            "Newer save intent was bypassed by deferred open",
        ));
    }
    println!("native-save-open-smoke: deferred=true stale-preserved=true committed=true restored=true dialogs=false");
    Ok(())
}

unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.process_restore(false)?;
        if Instant::now() >= deadline {
            app.cancel_restore();
            return Err(Error::new(E_FAIL, "Open workspace UI worker timed out"));
        }
        crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
    }
    Ok(())
}
