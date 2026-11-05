//! Real staged HWND placement and commit boundaries, without system clipboard access.
use super::*;
use crate::{
    desktop::{clipboard_editable::Content, clipboard_placement::Origin, image::Image},
    editor::Editor,
    pin_geometry::PinRect,
    workspace_id::UnitId,
};
use std::sync::Arc;

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let content = Arc::new(Content {
        image: Image {
            width: 8,
            height: 8,
            pixels: bytes(&[255; 256])?,
        },
        editor: Editor::default(),
        opacity: 1.0,
        capture_source: None,
        origin: Some(Origin {
            unit: UnitId::parse("retired-placement-source").unwrap(),
            frame: PinRect {
                x: -400,
                y: 80,
                width: 320,
                height: 160,
            },
            offset: (20, 20),
            step: 20,
        }),
    });
    let count = app.documents.len();
    app.editable_clipboard
        .borrow_mut()
        .published(77, Some(content.clone()));
    // Both requests are sampled before either is committed; the second must refresh its anchor.
    let request = app
        .editable_clipboard
        .borrow()
        .placement(&content, (-380, 100))
        .unwrap();
    for x in [-380, -360] {
        let mut prepared = content.prepare(|| false)?;
        prepared.placement = Some(request.clone());
        app.add_prepared(vec![prepared], false)?;
        let pin = app.documents.last().unwrap();
        if pin.state.canvas.borrow().image.is_none() {
            return Err(Error::new(
                E_FAIL,
                "Display-only clipboard snapshot lost its base",
            ));
        }
        let mut rect = RECT::default();
        GetWindowRect(pin.window.0, &mut rect)?;
        if rect
            != (RECT {
                left: x,
                top: 100 + (-380 - x).abs(),
                right: x + 320,
                bottom: 260 + (-380 - x).abs(),
            })
        {
            return Err(Error::new(
                E_FAIL,
                "Paste placement lost physical size or cascade",
            ));
        }
        let anchor = app
            .editable_clipboard
            .borrow()
            .placement(&content, (-380, 100))
            .unwrap();
        if anchor.unit() != &pin.state.unit_id {
            return Err(Error::new(
                E_FAIL,
                "Paste placement did not commit unit anchor",
            ));
        }
    }
    let before = app.editable_clipboard.borrow().refresh(&request);
    let mut failed = content.prepare(|| false)?;
    failed.placement = Some(before.clone());
    if app
        .add_prepared_with(vec![failed], false, |_, _, _, _| {
            Err(Error::new(E_FAIL, "Injected placement staging failure"))
        })
        .is_ok()
        || app.documents.len() != count + 2
        || app.editable_clipboard.borrow().refresh(&request).unit() != before.unit()
    {
        return Err(Error::new(E_FAIL, "Failed placement advanced the anchor"));
    }
    // A pending request may finish, but must never advance a newly copied session.
    app.editable_clipboard
        .borrow_mut()
        .published(78, Some(content.clone()));
    let mut stale = content.prepare(|| false)?;
    stale.placement = Some(before);
    app.add_prepared(vec![stale], false)?;
    if app
        .editable_clipboard
        .borrow()
        .placement(&content, (0, 0))
        .unwrap()
        .unit()
        != &content.origin.as_ref().unwrap().unit
    {
        return Err(Error::new(E_FAIL, "Old paste advanced a new copy session"));
    }
    for pin in &app.documents[count..] {
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
    }
    app.collect_closed();
    app.editable_clipboard.borrow_mut().published(0, None);
    if app.documents.len() != count {
        return Err(Error::new(
            E_FAIL,
            "Placement smoke left owned windows open",
        ));
    }
    println!("native-paste-placement-smoke: physical-size=true cascade=true pending-refresh=true failed-staging=true new-copy-isolated=true clipboard-access=false");
    Ok(())
}
