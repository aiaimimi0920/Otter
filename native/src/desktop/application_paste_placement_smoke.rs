//! Real staged HWND placement and commit boundaries, without system clipboard access.
use super::*;
use crate::{
    desktop::{clipboard_editable::Content, clipboard_placement::Origin, image::Image},
    editor::Editor,
    pin_geometry::PinRect,
    workspace_id::UnitId,
};
use std::sync::Arc;

unsafe fn moving_cursor_inside_source(app: &mut Application, source: usize) -> Result<()> {
    let doc = &app.documents[source];
    let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
    let origin = Origin::capture_at(doc.window.0, &doc.state, POINT { x: 630, y: 630 })
        .ok_or_else(|| Error::new(E_FAIL, "Cannot capture source cascade fixture"))?;
    let base = origin.frame;
    let step = origin.step;
    let content = Arc::new(Content {
        image: doc.state.canvas.borrow().image.clone().unwrap(),
        editor: Editor::default(),
        opacity: 1.0,
        frame_style: Default::default(),
        capture_source: None,
        origin: Some(origin),
    });
    app.editable_clipboard
        .borrow_mut()
        .published(79, Some(content.clone()));
    // 原 A 的任意位置都沿同一序列；可见副本覆盖 A 也不能改变复制来源身份。
    for (index, (dx, dy)) in [
        (1, 1),
        (3, 2),
        (470, 230),
        (80, 210),
        (150, 80),
        (200, 120),
        (300, 60),
        (1, 2),
        (460, 220),
        (20, 40),
        (240, 120),
        (2, 3),
    ]
    .into_iter()
    .enumerate()
    {
        let doc = &app.documents[source];
        crate::desktop::paste_input::deliver_at(
            doc.window.0,
            &doc.state,
            crate::desktop::paste_input::Snapshot::Editable(content.clone()),
            true,
            POINT {
                x: base.x + dx,
                y: base.y + dy,
            },
        )?;
        crate::desktop::open_job::finish(doc.window.0, &doc.state, true);
        app.pending_windows(false)?;
        let pin = app.documents.last().unwrap();
        let mut rect = RECT::default();
        GetWindowRect(pin.window.0, &mut rect)?;
        let expected = PinRect {
            x: base.x + step * (index as i32 + 1),
            y: base.y + step * (index as i32 + 1),
            ..base
        };
        if crate::desktop::clipboard_placement::rectangle(rect) != Some(expected) {
            return Err(Error::new(
                E_FAIL,
                format!(
                    "Source cascade step {}: expected ({}, {}), got ({}, {})",
                    index + 1,
                    expected.x,
                    expected.y,
                    rect.left,
                    rect.top,
                ),
            ));
        }
        SetWindowPos(
            pin.window.0,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
    }
    println!("native-paste-source-cascade-smoke: moving-cursor=true fixed-source=true overlapping-copies=true count=12 worker-delivery=true clipboard-access=false");
    Ok(())
}

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let content = Arc::new(Content {
        frame_style: Default::default(),
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
    // 排队请求共享采样坐标；越过鼠标后仍应沿每次成功落点继续推进。
    let request = app
        .editable_clipboard
        .borrow()
        .placement(&content, (-380, 100))
        .unwrap();
    for step in 1..=10 {
        let x = -400 + 20 * step;
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
        || app.documents.len() != count + 10
        || app.editable_clipboard.borrow().refresh(&request).unit() != before.unit()
    {
        return Err(Error::new(E_FAIL, "Failed placement advanced the anchor"));
    }
    // 在另一张已移动/缩放的钉图内粘贴，按其当前左上角；重叠时选最上层。
    for (index, x) in [(count, 600), (count + 1, 620)] {
        SetWindowPos(
            app.documents[index].window.0,
            Some(HWND_TOP),
            x,
            x,
            480,
            240,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
    }
    for (cursor, expected) in [(650, 640), (651, 620), (652, 632)] {
        let mut prepared = content.prepare(|| false)?;
        prepared.placement = app
            .editable_clipboard
            .borrow()
            .placement(&content, (cursor, cursor));
        app.add_prepared(vec![prepared], false)?;
        let mut rect = RECT::default();
        GetWindowRect(app.documents.last().unwrap().window.0, &mut rect)?;
        if rect
            != (RECT {
                left: expected,
                top: expected,
                right: expected + 320,
                bottom: expected + 160,
            })
        {
            return Err(Error::new(
                E_FAIL,
                "Paste did not anchor to the current topmost visible pin",
            ));
        }
        let _ = ShowWindow(app.documents[count + 1].window.0, SW_HIDE);
        if cursor == 651 {
            let _ = ShowWindow(app.documents[count].window.0, SW_HIDE);
        }
    }
    moving_cursor_inside_source(app, count)?;
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
    println!("native-paste-placement-smoke: physical-size=true cascade=true cascade-count=10 current-target=true topmost-target=true hidden-target-excluded=true pending-refresh=true failed-staging=true new-copy-isolated=true clipboard-access=false");
    Ok(())
}
