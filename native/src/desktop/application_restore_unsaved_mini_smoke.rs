//! 缺少 savedRect 的历史缩略窗口：真实 HWND 展开保持当前框，不猜测图片原尺寸。
use super::*;

pub(super) unsafe fn verify(app: &Application, catalog: &Rc<Loaded>) -> Result<()> {
    let cancel = AtomicBool::new(false);
    let candidate = crate::desktop::session_candidate::prepare(
        br#"{"stickers":[{"id":"unsaved-mini","x":-20.5,"y":10.5,"w":80,"h":60,"minified":true,"opacityNormal":0.4,"opacityMini":0.8,"cropOffset":{"x":12,"y":-4}}],"links":[]}"#.to_vec(),
        None,
        &cancel,
    )?;
    let bound = Bound::prepare(
        VectorCandidate::prepare(candidate, &cancel)?,
        catalog.clone(),
        &cancel,
    )
    .map_err(|failure| failure.1)?;
    let prepared = bound
        .into_parts(catalog, &cancel)
        .map_err(|_| Error::new(E_FAIL, "Synthetic catalog changed"))?
        .prepare_states(&cancel)?;
    let space = crate::session_placement::Space {
        origin: (-1920, -200),
        scale: 1.5,
    };
    let batch = app.prepare_restored_batch(prepared, &cancel, |placement| {
        space
            .physical(placement.rect)
            .map_err(|_| Error::new(E_FAIL, "Invalid unsaved mini placement"))
    })?;
    let doc = &batch.documents[0];
    let transition = doc
        .state
        .canvas
        .borrow()
        .minified_restore
        .ok_or_else(|| Error::new(E_FAIL, "Unsaved mini has no expansion transition"))?;
    let initial = transition.initial;
    if initial != transition.expanded || transition.persist_expanded {
        return Err(Error::new(E_FAIL, "Unsaved mini guessed expanded geometry"));
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
    let mut before = RECT::default();
    GetWindowRect(doc.window.0, &mut before)?;
    SendMessageW(
        doc.window.0,
        WM_NCLBUTTONDBLCLK,
        Some(WPARAM(HTCAPTION as usize)),
        Some(LPARAM(0)),
    );
    let mut after = RECT::default();
    GetWindowRect(doc.window.0, &mut after)?;
    let canvas = doc.state.canvas.borrow();
    if before != after
        || canvas.minified.is_some()
        || canvas.minified_restore.is_some()
        || canvas.window_opacity != 0.4
        || canvas.normal_opacity != 0.4
        || !stamp.matches(&canvas.editor)
        || IsWindowVisible(doc.window.0).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Unsaved mini expansion changed geometry or document",
        ));
    }
    println!("native-unsaved-mini-smoke: current-frame=true normal-opacity=true document-preserved=true hidden=true");
    Ok(())
}
