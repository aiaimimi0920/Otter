//! 使用独立隐藏 Application 验证实际 owner 交换；不触碰用户窗口、文件或全局热键。
use super::*;
#[path = "application_restore_empty_smoke.rs"]
mod empty;
use crate::desktop::application::restore_batch::smoke::{catalog, prepared};

pub(in crate::desktop::application::restore_batch) unsafe fn run(
    instance: HINSTANCE,
) -> Result<()> {
    empty::verify(instance)?;
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    let catalog = catalog()?;
    app.catalog.borrow_mut().loaded = Some(catalog.clone());
    let frame = crate::pin_geometry::PinRect {
        x: -200,
        y: 10,
        width: 100,
        height: 100,
    };
    let old = app.documents[0].window.0;
    let old_id = app.documents[0].state.unit_id.clone();
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let batch =
        app.prepare_restored_batch(prepared(&catalog)?, &AtomicBool::new(false), |_| Ok(frame))?;
    let candidates: Vec<_> = batch.documents.iter().map(|doc| doc.window.0).collect();
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
    if app.publish_restored_batch(batch, false).is_ok()
        || !IsWindow(Some(old)).as_bool()
        || app.source_window(&old_id) != Some(old)
        || app
            .restored_session
            .as_ref()
            .is_none_or(|record| !record.original.same_storage(&source))
        || candidates
            .iter()
            .any(|hwnd| IsWindow(Some(*hwnd)).as_bool())
    {
        return Err(Error::new(
            E_FAIL,
            "Stale restoration damaged original workspace",
        ));
    }
    let batch =
        app.prepare_restored_batch(prepared(&catalog)?, &AtomicBool::new(false), |_| Ok(frame))?;
    verify_hotkey_compensation(&app.documents, &batch.documents)?;
    let expected = batch.candidate.original_json().to_vec();
    let original_address = batch.candidate.original_json().as_ptr();
    let targets: Vec<_> = batch
        .documents
        .iter()
        .map(|doc| (doc.state.unit_id.clone(), doc.window.0))
        .collect();
    app.publish_restored_batch(batch, false)?;
    let record = app
        .restored_session
        .as_ref()
        .ok_or_else(|| Error::new(E_FAIL, "Missing session retention"))?;
    if IsWindow(Some(old)).as_bool()
        || app.workspace.contains(&old_id)
        || app.documents.len() != 2
        || app.workspace.units().len() != 2
        || record.original.as_ref() != expected
        || record.original.as_ptr() != original_address
        || record.annotation_ids.len() != 2
        || record.transient_host.is_some()
        || app.capture_windows.borrow().len() != 4
        || !app.executions.is_idle()
        || targets.iter().any(|(id, hwnd)| {
            app.source_window(id) != Some(*hwnd) || IsWindowVisible(*hwnd).as_bool()
        })
    {
        return Err(Error::new(
            E_FAIL,
            "Restoration publication owner/registry mismatch",
        ));
    }
    // 发布后的常规循环可使用新工作区，不能保留已释放旧 HWND 的缓存。
    app.pending_windows(false)?;
    if targets
        .iter()
        .any(|(id, hwnd)| app.source_window(id) != Some(*hwnd))
    {
        return Err(Error::new(
            E_FAIL,
            "Published workspace failed its first update",
        ));
    }
    drop(app);
    if targets
        .iter()
        .any(|(_, hwnd)| IsWindow(Some(*hwnd)).as_bool())
    {
        return Err(Error::new(E_FAIL, "Published document owner leaked HWND"));
    }
    println!("native-restored-publication-smoke: swapped=true stale-preserved=true retained-json=true cleanup=true");
    Ok(())
}

fn verify_hotkey_compensation(previous: &[DocumentWindow], next: &[DocumentWindow]) -> Result<()> {
    let states = [previous[0].state.as_ref(), next[0].state.as_ref()];
    let saved = states.map(|state| {
        (
            [
                state.global_capture.get(),
                state.global_live_capture.get(),
                state.global_long_capture.get(),
            ],
            [
                state.hotkey_enabled.get(),
                state.live_hotkey_enabled.get(),
                state.long_hotkey_enabled.get(),
            ],
        )
    });
    let change = |doc: &DocumentWindow, enabled| {
        crate::desktop::global_capture::update(&doc.state, enabled, |_, _| Ok(()))
    };
    let result = (|| -> Result<()> {
        change(&previous[0], true)?;
        let mut calls = Vec::new();
        let failed = transfer_hotkey(previous, next, |doc, enabled| {
            calls.push((doc.window.0, enabled));
            if doc.window.0 == next[0].window.0 {
                Err(Error::new(E_FAIL, "Injected hotkey transfer failure"))
            } else {
                change(doc, enabled)
            }
        });
        if failed.is_ok()
            || calls
                != [
                    (previous[0].window.0, false),
                    (next[0].window.0, true),
                    (previous[0].window.0, true),
                ]
            || !states[0].global_capture.get()
            || !states[0].global_live_capture.get()
            || !states[0].global_long_capture.get()
            || states[1].global_capture.get()
            || states[1].global_live_capture.get()
            || states[1].global_long_capture.get()
        {
            return Err(Error::new(
                E_FAIL,
                "Restoration hotkey compensation mismatch",
            ));
        }
        transfer_hotkey(previous, next, change)?;
        if states[0].global_capture.get()
            || states[0].global_live_capture.get()
            || states[0].global_long_capture.get()
            || !states[1].global_capture.get()
            || !states[1].global_live_capture.get()
            || !states[1].global_long_capture.get()
            || !states[1].live_hotkey_enabled.get()
            || !states[1].long_hotkey_enabled.get()
            || next
                .iter()
                .filter(|doc| doc.state.global_capture.get())
                .count()
                != 1
        {
            return Err(Error::new(
                E_FAIL,
                "Restoration lost unique secondary hotkey owner",
            ));
        }
        Ok(())
    })();
    // 即使验证失败，也在 HWND 析构前退休所有模拟注册状态；不向系统注销用户快捷键。
    for (state, (actual, shared)) in states.into_iter().zip(saved) {
        for (flag, value) in [
            &state.global_capture,
            &state.global_live_capture,
            &state.global_long_capture,
        ]
        .into_iter()
        .zip(actual)
        {
            flag.set(value);
        }
        for (flag, value) in [
            &state.hotkey_enabled,
            &state.live_hotkey_enabled,
            &state.long_hotkey_enabled,
        ]
        .into_iter()
        .zip(shared)
        {
            flag.set(value);
        }
    }
    result?;
    println!("native-restored-hotkey-transfer-smoke: live=true long=true rollback=true unique-owner=true system-registration=false");
    Ok(())
}
