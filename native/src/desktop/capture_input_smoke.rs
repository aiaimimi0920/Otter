//! 截图入口事务冒烟；注入现有图片，不调用桌面捕获、选择器或全局输入。
use super::*;

fn require(value: bool, message: &'static str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}

pub(in crate::desktop) unsafe fn shared(
    first: &crate::desktop::document_window::DocumentWindow,
    second: &crate::desktop::document_window::DocumentWindow,
) -> Result<()> {
    let windows = [
        first.window.0,
        first.panel.0,
        second.window.0,
        second.panel.0,
    ];
    require(
        windows.iter().all(|hwnd| !IsWindowVisible(*hwnd).as_bool()),
        "Shared capture smoke requires hidden fixtures",
    )?;
    {
        let guard = Hidden::for_state(first.window.0, &first.state);
        require(
            guard.0.len() == 4
                && windows
                    .iter()
                    .all(|hwnd| guard.0.iter().any(|entry| entry.0 == *hwnd)),
            "Capture snapshot omitted another document or duplicated a window",
        )?;
        require(
            windows.iter().all(|hwnd| !IsWindowVisible(*hwnd).as_bool()),
            "Capture snapshot revealed a hidden fixture",
        )?;
    }
    require(
        windows.iter().all(|hwnd| !IsWindowVisible(*hwnd).as_bool()),
        "Capture restoration revealed a previously hidden window",
    )?;
    {
        let _guard = crate::desktop::file_dialog::Guard::begin(&first.state.capture_active)
            .ok_or_else(|| Error::new(E_FAIL, "Capture gate already occupied"))?;
        require(
            crate::desktop::file_dialog::Guard::begin(&second.state.capture_active).is_none(),
            "Another document bypassed shared capture gate",
        )?;
    }
    require(
        !first.state.capture_active.get() && !second.state.capture_active.get(),
        "Capture gate did not release",
    )?;
    {
        let _modal = crate::desktop::file_dialog::Guard::begin(&second.state.file_dialog)
            .ok_or_else(|| Error::new(E_FAIL, "Fixture modal already occupied"))?;
        require(
            !first.state.file_dialog.get(),
            "Other document unexpectedly acquired local modal lock",
        )?;
        require(
            !exclusive_available(&first.state) && !exclusive_available(&second.state),
            "Screenshot bypassed another document's modal state",
        )?;
        SendMessageW(first.panel.0, WM_MOUSEACTIVATE, None, None);
        require(
            !IsWindowEnabled(GetDlgItem(Some(first.panel.0), DEFAULT as i32)?).as_bool(),
            "Default capture button ignored another document's modal gate",
        )?;
    }
    require(
        exclusive_available(&first.state) && exclusive_available(&second.state),
        "Screenshot remained blocked after modal release",
    )?;
    SendMessageW(first.panel.0, WM_MOUSEACTIVATE, None, None);
    require(
        IsWindowEnabled(GetDlgItem(Some(first.panel.0), DEFAULT as i32)?).as_bool(),
        "Default capture button stayed disabled after modal release",
    )?;
    println!("native-default-capture-smoke: new-pin=true modal-disabled=true recovered=true desktop-access=false");
    println!("native-shared-modal-smoke: capture-blocked=true local-independent=true released=true desktop-access=false");
    println!("native-shared-capture-smoke: windows=4 unique=true hidden-preserved=true shared-gate=true desktop-access=false");
    Ok(())
}

pub(in crate::desktop) unsafe fn run(hwnd: HWND, state: &State) -> Result<()> {
    let image = state
        .canvas
        .borrow()
        .image
        .clone()
        .ok_or_else(|| Error::new(E_FAIL, "Capture smoke requires fixture image"))?;
    let stamp = state.canvas.borrow().editor.document_stamp();
    run_with(hwnd, state, || {
        require(
            !IsWindowVisible(hwnd).as_bool(),
            "Capture host was not hidden",
        )?;
        require(
            !IsWindowVisible(state.tool_panel.get()).as_bool(),
            "Capture panel was not hidden",
        )?;
        Ok(None)
    })?;
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Cancelled capture changed document",
    )?;
    require(
        state.open_job.borrow().is_none(),
        "Cancelled capture started worker",
    )?;
    let failure = run_with(hwnd, state, || {
        Err(Error::new(E_FAIL, "Injected capture failure"))
    });
    require(failure.is_err(), "Capture failure was swallowed")?;
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Failed capture changed document",
    )?;
    require(
        !IsWindowVisible(hwnd).as_bool(),
        "Capture displayed originally hidden host",
    )?;
    require(
        !IsWindowVisible(state.tool_panel.get()).as_bool(),
        "Capture displayed hidden panel",
    )?;

    // 模拟选择器的嵌套消息循环换图；旧结果不得覆盖新文档，即使像素身份相同。
    let stale = run_with(hwnd, state, || {
        state.canvas.borrow_mut().replace_image(image.clone())?;
        Ok(Some(image.clone()))
    });
    require(
        stale.is_err_and(|error| error.code() == E_ABORT),
        "Stale capture accepted",
    )?;
    require(
        state.open_job.borrow().is_none(),
        "Stale capture started worker",
    )?;
    let stamp = state.canvas.borrow().editor.document_stamp();
    run_with(hwnd, state, || Ok(Some(image.clone())))?;
    require(
        state.open_job.borrow().is_some(),
        "Capture did not start worker",
    )?;
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Capture committed synchronously",
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while state.open_job.borrow().is_some() {
        require(
            std::time::Instant::now() < deadline,
            "Capture worker timeout",
        )?;
        let mut message = MSG::default();
        while PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if let Some(error) = state.error.borrow_mut().take() {
            return Err(error);
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    require(
        !stamp.matches(&state.canvas.borrow().editor),
        "Capture was not committed",
    )?;
    require(
        state
            .canvas
            .borrow()
            .image
            .as_ref()
            .is_some_and(|value| value.pixels.same_storage(&image.pixels)),
        "Capture copied or lost fixture pixels",
    )?;
    require(
        !IsWindowVisible(hwnd).as_bool(),
        "Commit displayed hidden host",
    )?;
    println!("native-capture-input-smoke: cancel=true failure=true stale=true timer=true desktop-access=false");
    Ok(())
}
