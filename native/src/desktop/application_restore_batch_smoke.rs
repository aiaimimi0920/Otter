//! 纯合成会话的隐藏窗口事务；固定占位目录不连接 Loom，不读取真实发现凭据。
use super::*;
#[path = "application_restore_readiness_smoke.rs"]
mod readiness;
#[path = "application_restore_unsaved_mini_smoke.rs"]
mod unsaved_mini;
use crate::desktop::session_binding::Bound;
use crate::desktop::session_candidate::VectorCandidate;

pub(in crate::desktop::application) fn catalog() -> Result<Rc<Loaded>> {
    use sha2::{Digest, Sha256};
    let identity = crate::loom_bridge_identity::Identity::validate(
        "loom.local-bridge.v1",
        "00000000-0000-0000-0000-000000000001",
        "wss://127.0.0.1:1/",
        "AA==",
        &format!("{:x}", Sha256::digest([0])),
        &"0".repeat(64),
    )
    .map_err(|_| Error::new(E_FAIL, "Invalid synthetic identity"))?;
    let handshake = crate::loom_handshake::response(br#"{"protocolVersion":"loom.hook.v1","serverName":"fixture","serverVersion":"1","sessionId":"fixture","transport":"websocket","capabilities":{"artDefinitions":[],"operations":[],"surface":{"apiVersion":"1","input":{"pointer":false,"hover":false,"touch":false,"keyboard":false},"runtimes":[],"nodes":[],"transports":[],"capabilities":[]}}}"#)
        .map_err(|_| Error::new(E_FAIL, "Invalid synthetic catalog"))?
        .ok_or_else(|| Error::new(E_FAIL, "Missing synthetic catalog"))?;
    Ok(Rc::new(Loaded {
        identity,
        handshake,
    }))
}
pub(super) fn prepared(catalog: &Rc<Loaded>) -> Result<Prepared> {
    let cancel = AtomicBool::new(false);
    let candidate = crate::desktop::session_candidate::prepare(
        br#"{"stickers":[{"id":"restore-a","x":0,"y":0,"w":10,"h":10},{"id":"restore-b","type":"art","artId":"missing","minified":true,"opacityMini":0.35,"savedRect":{"x":0,"y":0,"w":40,"h":30},"cropOffset":{"x":20,"y":0},"x":20,"y":0,"w":10,"h":10}],"links":[]}"#.to_vec(), None, &cancel)?;
    let bound = Bound::prepare(
        VectorCandidate::prepare(candidate, &cancel)?,
        catalog.clone(),
        &cancel,
    )
    .map_err(|failure| failure.1)?;
    bound
        .into_parts(catalog, &cancel)
        .map_err(|_| Error::new(E_FAIL, "Synthetic binding changed"))?
        .prepare_states(&cancel)
}
unsafe fn windows() -> Result<usize> {
    unsafe extern "system" fn count(_: HWND, data: LPARAM) -> windows::core::BOOL {
        *(data.0 as *mut usize) += 1;
        true.into()
    }
    let mut value = 0usize;
    EnumThreadWindows(
        windows::Win32::System::Threading::GetCurrentThreadId(),
        Some(count),
        LPARAM((&mut value as *mut usize) as isize),
    )
    .ok()?;
    Ok(value)
}
pub(in crate::desktop::application) unsafe fn run(app: &mut Application) -> Result<()> {
    use crate::desktop::smoke_timing::run as timed;
    timed("frozen-append", || {
        super::frozen_append::smoke::run(app.instance)
    })?;
    timed("restore-job", || super::job::smoke::verify(app.instance))?;
    timed("restore-publish", || {
        super::publish::smoke::run(app.instance)
    })?;
    let catalog = catalog()?;
    let previous = app.catalog.borrow_mut().loaded.replace(catalog.clone());
    let result = readiness::verify_requests(app, &catalog).and_then(|_| verify(app, &catalog));
    app.catalog.borrow_mut().loaded = previous;
    result
}
unsafe fn verify(app: &Application, catalog: &Rc<Loaded>) -> Result<()> {
    unsaved_mini::verify(app, catalog)?;
    readiness::reject_inconsistent(app, catalog)?;
    let before = windows()?;
    let count = app.documents.len();
    let frame = crate::pin_geometry::PinRect {
        x: -100,
        y: 60,
        width: 160,
        height: 100,
    };
    let space = crate::session_placement::Space {
        origin: (-1920, -200),
        scale: 1.5,
    };
    let batch =
        app.prepare_restored_batch(prepared(catalog)?, &AtomicBool::new(false), |placement| {
            space
                .physical(placement.rect)
                .map_err(|_| Error::new(E_FAIL, "Synthetic coordinate conversion failed"))
        })?;
    if batch.window_count() != 2
        || !batch.matches_application(app)
        || batch.annotation_ids.len() != 2
        || batch.candidate.workspace.units().len() != 2
        || !Rc::ptr_eq(batch.catalog.as_ref().unwrap(), catalog)
        || batch
            .documents
            .iter()
            .any(|doc| IsWindowVisible(doc.window.0).as_bool())
    {
        return Err(Error::new(E_FAIL, "Restored batch identity mismatch"));
    }
    let source = &app.documents[0].state;
    // Ctrl+2 实际注册和共享降级状态必须进入原工作区与隐藏候选的恢复凭证。
    for flag in [
        &source.global_live_capture,
        &app.live_hotkey_enabled,
        &batch.documents[0].state.global_live_capture,
    ] {
        let previous = flag.replace(!flag.get());
        let stale = !batch.matches_application(app);
        flag.set(previous);
        if !stale || !batch.matches_application(app) {
            return Err(Error::new(
                E_FAIL,
                "Restoration ignored changed Live hotkey state",
            ));
        }
    }
    for doc in &batch.documents {
        let expected_opacity = if doc.state.unit_id.as_str() == "restore-b" {
            0.35
        } else {
            1.0
        };
        if doc.state.canvas.borrow().window_opacity != expected_opacity
            || doc.state.canvas.borrow().normal_opacity != 1.0
        {
            return Err(Error::new(
                E_FAIL,
                "Restored window opacity selection mismatch",
            ));
        }
        let placement = batch
            .candidate
            .placements
            .iter()
            .find(|item| item.unit == doc.state.unit_id)
            .unwrap();
        let expected = space.physical(placement.rect).unwrap();
        let mut actual = RECT::default();
        GetWindowRect(doc.window.0, &mut actual)?;
        if actual.left != expected.x
            || actual.top != expected.y
            || actual.right - actual.left != expected.width as i32
            || actual.bottom - actual.top != expected.height as i32
        {
            return Err(Error::new(
                E_FAIL,
                "Restored CSS placement did not reach physical HWND",
            ));
        }
    }
    let accept = source.accept_upstream.get();
    source.accept_upstream.set(!accept);
    let stale = !batch.matches_application(app);
    source.accept_upstream.set(accept);
    if !stale || !batch.matches_application(app) {
        return Err(Error::new(
            E_FAIL,
            "Restored batch did not track source configuration",
        ));
    }
    source
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point::default(),
        )
        .map_err(|_| Error::new(E_FAIL, "Cannot prepare synthetic draft"))?;
    let stale = !batch.matches_application(app);
    source.canvas.borrow_mut().editor.cancel();
    if !stale || !batch.matches_application(app) {
        return Err(Error::new(
            E_FAIL,
            "Restored batch accepted active source draft",
        ));
    }
    source.new_window.set(true);
    let stale = !batch.matches_application(app);
    source.new_window.set(false);
    if !stale {
        return Err(Error::new(
            E_FAIL,
            "Restored batch accepted queued window request",
        ));
    }
    let original_display = source.resolved_display.borrow_mut().take();
    *source.resolved_display.borrow_mut() = Some(crate::desktop::display_frame::Frame::capture(
        source.unit_id.clone(),
        crate::desktop::image::Image {
            width: 1,
            height: 1,
            pixels: crate::pixels::Pixels::from_reserved(
                vec![255; 4],
                crate::pixels::reserve(4)
                    .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Synthetic pixel budget"))?,
            ),
        },
        None,
    ));
    let stale = !batch.matches_application(app);
    *source.resolved_display.borrow_mut() = original_display;
    if !stale || !batch.matches_application(app) {
        return Err(Error::new(
            E_FAIL,
            "Restored batch did not track resolved display",
        ));
    }
    readiness::verify(app, &batch)?;
    verify_expand(&batch)?;
    if batch.matches_application(app) {
        return Err(Error::new(
            E_FAIL,
            "Changed candidate geometry retained restoration readiness",
        ));
    }
    drop(batch);
    let mut converted = 0;
    let failed = app.prepare_restored_batch(prepared(catalog)?, &AtomicBool::new(false), |_| {
        converted += 1;
        if converted == 2 {
            Err(Error::new(E_FAIL, "Injected placement failure"))
        } else {
            Ok(frame)
        }
    });
    if failed.is_ok() || converted != 2 || windows()? != before || app.documents.len() != count {
        return Err(Error::new(
            E_FAIL,
            "Partial restored windows were not rolled back",
        ));
    }
    let cancel = AtomicBool::new(false);
    let mut converted = 0;
    let cancelled = app.prepare_restored_batch(prepared(catalog)?, &cancel, |_| {
        converted += 1;
        if converted == 2 {
            cancel.store(true, Ordering::Release);
        }
        Ok(frame)
    });
    if cancelled.is_ok() || windows()? != before || app.modals.blocked() {
        return Err(Error::new(E_FAIL, "Cancelled restored windows leaked"));
    }
    println!("native-restored-batch-smoke: hidden=true partial-failure-cleanup=true cancelled-cleanup=true");
    Ok(())
}

unsafe fn verify_expand(batch: &Windows) -> Result<()> {
    let doc = &batch.documents[1];
    let transition = doc
        .state
        .canvas
        .borrow()
        .minified_restore
        .ok_or_else(|| Error::new(E_FAIL, "Missing restored expansion target"))?;
    let (initial, expanded, opacity) =
        (transition.initial, transition.expanded, transition.opacity);
    if !transition.persist_expanded {
        return Err(Error::new(
            E_FAIL,
            "Saved expansion lost persistence identity",
        ));
    }
    let stamp = doc.state.canvas.borrow().editor.document_stamp();
    SetWindowPos(
        doc.window.0,
        None,
        initial.x + 17,
        initial.y - 9,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )?;
    doc.state.group_locked.set(true);
    SendMessageW(
        doc.window.0,
        WM_NCLBUTTONDBLCLK,
        Some(WPARAM(HTCAPTION as usize)),
        Some(LPARAM(0)),
    );
    if doc.state.canvas.borrow().minified.is_none() {
        return Err(Error::new(E_FAIL, "Locked minified window expanded"));
    }
    doc.state.group_locked.set(false);
    SendMessageW(
        doc.window.0,
        WM_NCLBUTTONDBLCLK,
        Some(WPARAM(HTCAPTION as usize)),
        Some(LPARAM(0)),
    );
    let mut actual = RECT::default();
    GetWindowRect(doc.window.0, &mut actual)?;
    let canvas = doc.state.canvas.borrow();
    if actual.left != expanded.x + 17
        || actual.top != expanded.y - 9
        || actual.right - actual.left != expanded.width as i32
        || actual.bottom - actual.top != expanded.height as i32
        || canvas.minified.is_some()
        || canvas.minified_restore.is_some()
        || canvas.window_opacity != opacity
        || !stamp.matches(&canvas.editor)
        || IsWindowVisible(doc.window.0).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Hidden minified double-click expansion failed",
        ));
    }
    Ok(())
}
