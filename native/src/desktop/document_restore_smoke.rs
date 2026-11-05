//! 启动 smoke 的隐藏恢复窗口验证；不用用户图片、剪贴板、真实会话或全局热键。
use super::*;
#[path = "minified_display_region_smoke.rs"]
mod display_region;

pub(super) unsafe fn verify(existing: HWND) -> Result<()> {
    let instance = HINSTANCE(GetWindowLongPtrW(existing, GWLP_HINSTANCE) as *mut core::ffi::c_void);
    let frame = crate::pin_geometry::PinRect {
        x: -120,
        y: 60,
        width: 320,
        height: 180,
    };
    let document = DocumentWindow::create_at(instance, State::new(None, true)?, frame)?;
    let hwnd = document.window.0;
    let panel = document.panel.0;
    let mut actual = RECT::default();
    GetWindowRect(hwnd, &mut actual)?;
    if !document.is_alive()
        || IsWindowVisible(hwnd).as_bool()
        || IsWindowVisible(panel).as_bool()
        || actual.left != frame.x
        || actual.top != frame.y
        || actual.right - actual.left != frame.width as i32
        || actual.bottom - actual.top != frame.height as i32
        || document.state.canvas.borrow().image.is_some()
        || document.state.priority_watcher.borrow().is_some()
    {
        return Err(Error::new(
            E_FAIL,
            "Hidden restored window geometry/state mismatch",
        ));
    }
    document.icons.verify(hwnd)?;
    document.icons.verify(panel)?;
    SendMessageW(hwnd, WM_NULL, None, None);
    drop(document);
    if IsWindow(Some(hwnd)).as_bool()
        || IsWindow(Some(panel)).as_bool()
        || !IsWindow(Some(existing)).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Restored window cleanup affected owner lifetime",
        ));
    }
    for invalid in [
        crate::pin_geometry::PinRect { width: 0, ..frame },
        crate::pin_geometry::PinRect {
            width: 8193,
            ..frame
        },
        crate::pin_geometry::PinRect {
            x: i32::MAX,
            ..frame
        },
    ] {
        if DocumentWindow::create_at(instance, State::new(None, true)?, invalid).is_ok() {
            return Err(Error::new(E_FAIL, "Invalid restored window frame accepted"));
        }
    }
    verify_toggle(instance, frame, true)?;
    verify_toggle(instance, frame, false)?;
    println!("native-restored-window-smoke: hidden=true no-image=true placement=true cleanup=true");
    Ok(())
}

unsafe fn verify_toggle(
    instance: HINSTANCE,
    frame: crate::pin_geometry::PinRect,
    local: bool,
) -> Result<()> {
    let bytes = vec![255; 32 * 18 * 4];
    let lease = crate::pixels::reserve(bytes.len())
        .ok_or_else(|| Error::new(E_FAIL, "Synthetic image budget"))?;
    let image = crate::desktop::image::Image {
        width: 32,
        height: 18,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    };
    let original = crate::erase_layers::Layers {
        size: (image.width, image.height),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    };
    let document =
        DocumentWindow::create_at(instance, State::new(local.then_some(image), true)?, frame)?;
    let hwnd = document.window.0;
    let stamp = document.state.canvas.borrow().editor.document_stamp();
    let local_before = document.state.canvas.borrow().editor.layers().cloned();
    for cycle in 0..2 {
        if !local || cycle == 1 {
            *document.state.resolved_display.borrow_mut() = Some(synthetic_display(&original));
        }
        let display_identity = document
            .state
            .resolved_display
            .borrow()
            .as_ref()
            .map(|frame| frame.identity.clone());
        document.state.canvas.borrow_mut().tool = crate::editor::Tool::Pan;
        let mut before = RECT::default();
        GetWindowRect(hwnd, &mut before)?;
        let x = before.left + 20;
        let y = before.top + 20;
        let point = LPARAM(((y as u16 as u32) << 16 | x as u16 as u32) as isize);
        SendMessageW(
            hwnd,
            WM_NCLBUTTONDBLCLK,
            Some(WPARAM(HTCAPTION as usize)),
            Some(point),
        );
        let transition = document
            .state
            .canvas
            .borrow()
            .minified_restore
            .ok_or_else(|| Error::new(E_FAIL, "Normal pin did not minify"))?;
        if document.state.canvas.borrow().minified.is_none() {
            return Err(Error::new(E_FAIL, "Missing minified presentation"));
        }
        if let Some(identity) = display_identity {
            if !document
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .is_some_and(|frame| std::sync::Arc::ptr_eq(&frame.identity, &identity))
            {
                return Err(Error::new(
                    E_FAIL,
                    "Minify replaced external display identity",
                ));
            }
        }
        document.state.canvas.borrow_mut().tool = crate::editor::Tool::Brush;
        SendMessageW(
            hwnd,
            WM_LBUTTONDOWN,
            Some(WPARAM(0)),
            Some(LPARAM(4 | (4 << 16))),
        );
        SendMessageW(
            hwnd,
            WM_LBUTTONUP,
            Some(WPARAM(0)),
            Some(LPARAM(4 | (4 << 16))),
        );
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x2E)), Some(LPARAM(0)));
        if document.state.canvas.borrow().editor.draft().is_some()
            || !stamp.matches(&document.state.canvas.borrow().editor)
            || crate::desktop::display_edit::prepare(&document.state)
            || crate::desktop::display_edit::prepare_content(&document.state)
            || crate::desktop::display_edit::prepare_crop(&document.state)
            || SendMessageW(hwnd, WM_NCHITTEST, None, Some(point)).0 != HTCAPTION as isize
        {
            return Err(Error::new(
                E_FAIL,
                "Minified pin accepted annotation edits or lost drag hit target",
            ));
        }
        document.state.file_dialog.set(true);
        SendMessageW(
            hwnd,
            WM_NCLBUTTONDBLCLK,
            Some(WPARAM(HTCAPTION as usize)),
            Some(point),
        );
        let blocked =
            document.state.canvas.borrow().minified.is_some() && document.state.file_dialog.get();
        document.state.file_dialog.set(false);
        if !blocked {
            return Err(Error::new(
                E_FAIL,
                "Busy window expanded or consumed pending dialog state",
            ));
        }
        let import = || {
            crate::desktop::open_job::Input::Image(crate::desktop::image::Image {
                width: original.size.0,
                height: original.size.1,
                pixels: original.base.clone(),
            })
        };
        if crate::desktop::open_job::start_destination(hwnd, &document.state, import(), false)
            .is_ok()
            || document.state.open_job.borrow().is_some()
        {
            return Err(Error::new(
                E_FAIL,
                "Minified replacement unexpectedly started worker",
            ));
        }
        crate::desktop::open_job::start_destination(hwnd, &document.state, import(), true)?;
        crate::desktop::open_job::finish(hwnd, &document.state, true);
        let created = document
            .state
            .new_pin
            .borrow_mut()
            .take()
            .ok_or_else(|| Error::new(E_FAIL, "Minified host blocked new pin import"))?;
        if created.len() != 1
            || !created[0].image.pixels.same_storage(&original.base)
            || document.state.canvas.borrow().minified.is_none()
            || !stamp.matches(&document.state.canvas.borrow().editor)
        {
            return Err(Error::new(E_FAIL, "New pin import modified minified host"));
        }
        *document.state.resolved_display.borrow_mut() = Some(synthetic_display(&original));
        display_region::verify(hwnd, &document.state)?;
        *document.state.resolved_display.borrow_mut() = Some(synthetic_display(&original));
        crate::desktop::hit_region::update(hwnd, &document.state)?;
        {
            let display = document.state.resolved_display.borrow();
            document.state.canvas.borrow_mut().paint_display(
                hwnd,
                transition.initial.width,
                transition.initial.height,
                windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
                display.as_ref(),
            )?;
        }
        SetWindowPos(
            hwnd,
            None,
            transition.initial.x + 17,
            transition.initial.y - 9,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
        SendMessageW(
            hwnd,
            WM_NCLBUTTONDBLCLK,
            Some(WPARAM(HTCAPTION as usize)),
            Some(point),
        );
        let mut after = RECT::default();
        GetWindowRect(hwnd, &mut after)?;
        if document
            .state
            .resolved_display
            .borrow_mut()
            .take()
            .is_none()
        {
            return Err(Error::new(
                E_FAIL,
                "Minified toggle consumed external display",
            ));
        }
        let canvas = document.state.canvas.borrow();
        if canvas.minified.is_some()
            || canvas.minified_restore.is_some()
            || after.left != before.left + 17
            || after.top != before.top - 9
            || after.right - after.left != frame.width as i32
            || after.bottom - after.top != frame.height as i32
            || !stamp.matches(&canvas.editor)
            || match (&local_before, canvas.editor.layers()) {
                (None, None) => false,
                (Some(before), Some(after)) => !before.same_storage(after),
                _ => true,
            }
            || canvas.window_opacity != 1.0
            || IsWindowVisible(hwnd).as_bool()
        {
            return Err(Error::new(
                E_FAIL,
                format!("Native minify/restore cycle {cycle} changed content or placement"),
            ));
        }
    }
    Ok(())
}
fn synthetic_display(source: &crate::erase_layers::Layers) -> crate::desktop::display_frame::Frame {
    crate::desktop::display_frame::Frame::capture(
        crate::workspace_id::UnitId::parse("synthetic-display").unwrap(),
        crate::desktop::image::Image {
            width: source.size.0,
            height: source.size.1,
            pixels: source.base.clone(),
        },
        None,
    )
}
