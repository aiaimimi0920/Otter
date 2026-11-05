//! 自建历史条目经过生产准入和交付；真实窗口/后台解码，不访问系统历史。
use super::*;
use crate::{
    clipboard_history::{Content, Entry},
    desktop::{open_job, popup_menu},
};

fn text() -> Entry {
    Entry {
        id: "local-text-fixture".into(),
        content: Content::Text(format!("第一行\n\nsecond 😀\n{}", "x".repeat(5000))),
    }
}
fn bitmap() -> Result<Entry> {
    let bytes = super::image_fixture::PNG;
    let image = crate::image_decode::Image::from_clipboard_history(bytes)?;
    let lease = crate::pixels::reserve(bytes.len())
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "History fixture budget exhausted"))?;
    Ok(Entry {
        id: "local-bitmap-fixture".into(),
        content: Content::Bitmap {
            width: image.width,
            height: image.height,
            thumbnail: crate::image_thumbnail::Thumbnail::from_image(&image)?,
            encoded: crate::pixels::Pixels::from_reserved(bytes.to_vec(), lease),
        },
    })
}

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let source = &app.documents[0];
    let hwnd = source.window.0;
    let state = &source.state;
    let (center, original_count, stamp) = {
        let canvas = state.canvas.borrow();
        let image = canvas.image.as_ref().unwrap();
        let center = canvas.editor.annotation_frame().map_or(
            (image.width as f32 / 2.0, image.height as f32 / 2.0),
            |rect| {
                (
                    rect.x as f32 + rect.width as f32 / 2.0,
                    rect.y as f32 + rect.height as f32 / 2.0,
                )
            },
        );
        (
            center,
            canvas.editor.elements().len(),
            canvas.editor.document_stamp(),
        )
    };
    {
        let _guard = crate::desktop::file_dialog::Guard::begin(&state.file_dialog)
            .ok_or_else(|| Error::new(E_FAIL, "History fixture modal gate already busy"))?;
        popup_menu::deliver_history(hwnd, state, text(), || {
            panic!("Busy gate must reject first")
        })?;
    }
    if !stamp.matches(&state.canvas.borrow().editor) {
        return Err(Error::new(E_FAIL, "Busy history delivery changed source"));
    }
    popup_menu::deliver_history(hwnd, state, text(), || {
        stamp.matches(&state.canvas.borrow().editor)
    })?;
    {
        let canvas = state.canvas.borrow();
        let rows = canvas.editor.elements();
        if rows.len() != original_count + 3
            || rows[original_count]
                .text
                .as_ref()
                .map(|text| text.content())
                != Some("第一行")
            || rows[original_count + 1]
                .text
                .as_ref()
                .map(|text| text.content())
                != Some("second 😀")
            || rows[original_count + 2]
                .text
                .as_ref()
                .map(|text| text.content().len())
                != Some(5000)
        {
            return Err(Error::new(
                E_FAIL,
                "History text did not reach the actual document",
            ));
        }
        let imported = &rows[original_count..];
        if imported.iter().any(|row| {
            row.points.len() != 2
                || ((row.points[0].x + row.points[1].x) / 2.0 - center.0).abs() > 0.01
        }) || ((imported[0].points[0].y + imported[2].points[1].y) / 2.0 - center.1).abs() > 0.01
        {
            return Err(Error::new(
                E_FAIL,
                "History text is not centered in the actual document",
            ));
        }
    }
    popup_menu::deliver_history(hwnd, state, bitmap()?, || {
        stamp.matches(&state.canvas.borrow().editor)
    })?;
    if state.open_job.borrow().is_some() || state.new_pin.borrow().is_some() {
        return Err(Error::new(
            E_FAIL,
            "Stale history target started image delivery",
        ));
    }
    {
        let mut canvas = state.canvas.borrow_mut();
        if !canvas.editor.undo() || canvas.editor.elements().len() != original_count {
            return Err(Error::new(
                E_FAIL,
                "History text import was not one undo step",
            ));
        }
    }
    let stamp = state.canvas.borrow().editor.document_stamp();
    popup_menu::deliver_history(hwnd, state, bitmap()?, || {
        stamp.matches(&state.canvas.borrow().editor)
    })?;
    open_job::finish(hwnd, state, true);
    if !stamp.matches(&state.canvas.borrow().editor) || state.new_pin.borrow().is_none() {
        return Err(Error::new(
            E_FAIL,
            "History bitmap replaced source or lost pending pin",
        ));
    }
    let count = app.documents.len();
    app.pending_windows(false)?;
    if app.documents.len() != count + 1 {
        return Err(Error::new(
            E_FAIL,
            "History bitmap did not create an independent window",
        ));
    }
    let pin = app.documents.last().unwrap();
    {
        let canvas = pin.state.canvas.borrow();
        if !canvas.pinned
            || canvas
                .image
                .as_ref()
                .map(|image| (image.width, image.height))
                != Some((32, 32))
        {
            return Err(Error::new(
                E_FAIL,
                "History pin has wrong image or window mode",
            ));
        }
    }
    SendMessageW(pin.window.0, WM_CLOSE, None, None);
    app.collect_closed();
    println!("native-history-delivery-smoke: text=true single-undo=true stale-rejected=true modal-rejected=true independent-pin=true clipboard-access=false");
    Ok(())
}
