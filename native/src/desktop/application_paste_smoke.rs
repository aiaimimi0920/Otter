//! 剪贴板格式快照到独立窗口的真实线程链；不打开或修改系统剪贴板。
use super::*;
#[path = "application_paste_editable_smoke.rs"]
mod editable;
#[path = "application_paste_placement_smoke.rs"]
mod placement;
use crate::desktop::{
    open_job,
    paste_input::{self, Snapshot},
};

fn bytes(input: &[u8]) -> Result<crate::pixels::Pixels> {
    let lease = crate::pixels::reserve(input.len())
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Paste fixture budget exhausted"))?;
    Ok(crate::pixels::Pixels::from_reserved(input.to_vec(), lease))
}

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    editable::run(app)?;
    placement::run(app)?;
    let png = smoke::image_fixture::PNG;
    let mut dib = [0u8; 44];
    dib[..4].copy_from_slice(&40u32.to_le_bytes());
    dib[4..8].copy_from_slice(&1i32.to_le_bytes());
    dib[8..12].copy_from_slice(&1i32.to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    dib[40..44].copy_from_slice(&[10, 20, 30, 255]);
    let fixture = smoke::image_fixture::ImageFile::new()?;
    let path = fixture.path().to_owned();
    for (snapshot, dimensions) in [
        (Snapshot::Png(bytes(png)?), (32, 32)),
        (Snapshot::Dib(bytes(&dib)?), (1, 1)),
        (Snapshot::Files(vec![path.clone()]), (32, 32)),
    ] {
        let count = app.documents.len();
        let source = &app.documents[0];
        let stamp = source.state.canvas.borrow().editor.document_stamp();
        paste_input::deliver(source.window.0, &source.state, snapshot, true)?;
        // 强制等待真实工作线程结束；不调用桌面/剪贴板 API，也不显示对话框。
        open_job::finish(source.window.0, &source.state, true);
        if !stamp.matches(&source.state.canvas.borrow().editor)
            || source.state.new_pin.borrow().is_none()
        {
            return Err(Error::new(E_FAIL, "Paste lost pin or replaced source"));
        }
        if paste_input::deliver(
            source.window.0,
            &source.state,
            Snapshot::Png(bytes(png)?),
            true,
        )
        .is_ok()
        {
            return Err(Error::new(E_FAIL, "Paste overwrote pending pin"));
        }
        app.pending_windows(false)?;
        if app.documents.len() != count + 1 {
            return Err(Error::new(E_FAIL, "Paste did not create a document"));
        }
        let pin = app.documents.last().unwrap();
        let canvas = pin.state.canvas.borrow();
        if !canvas.pinned
            || !canvas.editor.elements().is_empty()
            || canvas
                .image
                .as_ref()
                .map(|image| (image.width, image.height))
                != Some(dimensions)
        {
            return Err(Error::new(
                E_FAIL,
                "Paste pin has wrong image or editing state",
            ));
        }
        drop(canvas);
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
        app.collect_closed();
    }
    let source = &app.documents[0];
    let stamp = source.state.canvas.borrow().editor.document_stamp();
    let paths = vec![path.clone(), path];
    if paste_input::deliver(
        source.window.0,
        &source.state,
        Snapshot::Files(paths.clone()),
        false,
    )
    .is_ok()
        || source.state.open_job.borrow().is_some()
    {
        return Err(Error::new(
            E_FAIL,
            "Replacement accepted multi-file clipboard",
        ));
    }
    paste_input::deliver(source.window.0, &source.state, Snapshot::Files(paths), true)?;
    open_job::finish(source.window.0, &source.state, true);
    let count = app.documents.len();
    app.pending_windows(false)?;
    if app.documents.len() != count + 2
        || !stamp.matches(&app.documents[0].state.canvas.borrow().editor)
    {
        return Err(Error::new(
            E_FAIL,
            "Clipboard batch lost windows or changed source",
        ));
    }
    for pin in &app.documents[count..] {
        if !pin.state.canvas.borrow().pinned {
            return Err(Error::new(E_FAIL, "Clipboard batch did not create pins"));
        }
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
    }
    app.collect_closed();
    let source = &app.documents[0];
    paste_input::deliver(
        source.window.0,
        &source.state,
        Snapshot::Png(bytes(png)?),
        true,
    )?;
    open_job::cancel(&source.state);
    open_job::finish(source.window.0, &source.state, true);
    if source.state.new_pin.borrow().is_some()
        || !stamp.matches(&source.state.canvas.borrow().editor)
    {
        return Err(Error::new(E_FAIL, "Cancelled paste published a document"));
    }
    paste_input::deliver(
        source.window.0,
        &source.state,
        Snapshot::Png(bytes(png)?),
        false,
    )?;
    open_job::finish(source.window.0, &source.state, true);
    if source.state.new_pin.borrow().is_some()
        || stamp.matches(&source.state.canvas.borrow().editor)
    {
        return Err(Error::new(
            E_FAIL,
            "Explicit paste replacement created a pin or failed to replace",
        ));
    }
    println!("native-paste-pin-smoke: png=true dib=true file=true source-preserved=true pending-bounded=true cancelled=true clipboard-access=false");
    println!("native-paste-batch-smoke: count=2 source-preserved=true replacement-rejected=true clipboard-access=false");
    Ok(())
}
