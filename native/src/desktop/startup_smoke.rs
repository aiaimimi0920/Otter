//! 隐藏启动验收序列；复用生产 owner，不拥有 HWND、期限或系统输入。
use super::*;

pub(super) unsafe fn run(
    instance: HINSTANCE,
    document: &document_window::DocumentWindow,
    image_path: Option<&Path>,
    settings: &settings_startup::Loaded,
    shader: startup_entry::shader::Loaded,
) -> Result<()> {
    let state = &*document.state;
    let window = &document.window;
    let _tool_panel = &document.panel;
    let icons = &document.icons;
    let pinned = state.canvas.borrow().pinned;
    if let Some(settings) = &settings.source {
        settings_startup::verify(state, settings.document())?;
        println!("native-settings-startup: explicit=true applied=true source-readonly=true");
    }
    smoke_timing::run("application", || application::smoke(instance, shader))?;
    smoke_timing::run("document-owner-smoke", || {
        document_window_smoke::run(instance, document)
    })?;
    icons.verify(window.0)?;
    icons.verify(_tool_panel.0)?;
    println!("native-icon-smoke: host=true panel=true embedded=true");
    smoke_timing::run("tool-panel-smoke", || tool_panel::smoke(window.0, state))?;
    smoke_timing::run("capture-protection-smoke", || {
        capture_protection::smoke(window.0, state)
    })?;
    if let Some(path) = image_path {
        smoke_timing::run("file-drop-smoke", || {
            file_drop_smoke::run(window.0, state, path)
        })?;
        smoke_timing::run("capture-input-smoke", || {
            capture_input::smoke(window.0, state)
        })?;
    }
    let annotations = {
        use crate::{editor::Tool, viewport::Point};
        let mut canvas = state.canvas.borrow_mut();
        if canvas.image.is_some() {
            let image = canvas.image.as_ref().unwrap();
            let view = canvas.viewport;
            let start = Point {
                x: view.origin.x + image.width as f32 * view.scale * 0.25,
                y: view.origin.y + image.height as f32 * view.scale * 0.25,
            };
            let end = Point {
                x: view.origin.x + image.width as f32 * view.scale * 0.75,
                y: view.origin.y + image.height as f32 * view.scale * 0.75,
            };
            for tool in [Tool::Brush, Tool::Line, Tool::Rectangle, Tool::Ellipse] {
                canvas.select_tool(tool);
                canvas.pointer_down(start);
                canvas.pointer_up(end);
            }
            if canvas.editor.elements().len() != 4 {
                return Err(Error::new(
                    E_FAIL,
                    "Native edit smoke did not commit four tools",
                ));
            }
        }
        canvas.editor.elements().len()
    };
    if pinned {
        smoke_timing::run("crop-smoke", || crop_smoke::run(window.0, state))?;
    }
    if state.canvas.borrow().image.is_some() {
        smoke_timing::run("flip-smoke", || flip_smoke::run(window.0, state))?;
        smoke_timing::run("crop-effect-smoke", || {
            crop_effect_smoke::run(window.0, state)
        })?;
        smoke_timing::run("copy-smoke", || copy_smoke::run(window.0, state))?;
        smoke_timing::run("marquee-smoke", || marquee_smoke::run(window.0, state))?;
    }
    let before_paints = state.canvas.borrow().paints;
    // 隐藏窗口也真实创建 D2D target 并提交像素；不是人工显示验收。
    smoke_timing::run("final-first-paint", || {
        let _ = SendMessageW(window.0, WM_PAINT, None, None);
        Ok::<(), Error>(())
    })?;
    // 模拟设备资源失效；模型和 CPU 像素必须能在下一次绘制重建呈现资源。
    smoke_timing::run("final-discard-device", || {
        state.canvas.borrow_mut().discard_device();
        Ok::<(), Error>(())
    })?;
    state
        .canvas
        .borrow_mut()
        .select_tool(crate::editor::Tool::Pan);
    smoke_timing::run("final-rebuild-paint", || {
        let _ = SendMessageW(window.0, WM_PAINT, None, None);
        Ok::<(), Error>(())
    })?;
    let paints = state.canvas.borrow().paints;
    if state.error.borrow().is_none() {
        smoke_timing::run("final-retirement", || retire_canvas(state))?;
    }
    smoke_timing::run("final-destroy-window", || DestroyWindow(window.0))?;
    document_window_smoke::verify_last_exit()?;
    if let Some(error) = state.error.borrow_mut().take() {
        return Err(error);
    }
    if paints != before_paints + 2
        || IsWindow(Some(window.0)).as_bool()
        || IsWindow(Some(_tool_panel.0)).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Native window paint/lifecycle smoke failed",
        ));
    }
    println!("native-smoke: paints={paints} destroyed=true webview=false");
    println!("native-rebuild-smoke: paints={}", paints - before_paints);
    println!("native-edit-smoke: annotations={annotations} device-recreated=true");
    println!("native-window-smoke: pinned={pinned}");
    Ok(())
}

fn retire_canvas(state: &State) -> Result<()> {
    use super::composition_retirement::Status;
    let (stamp, paints, confirmed) = {
        let canvas = state.canvas.borrow();
        (
            canvas.editor.document_stamp(),
            canvas.paints,
            canvas.confirmed_paints,
        )
    };
    let mut has_composition = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        // 仅隐藏启动自检允许在此轮询；默认消息循环/WM_PAINT 不调用此等待流程。
        // 不分发待绘制消息；退休 owner 两秒后进入终态，不代表系统等待已被终止。
        // 自检另设外层上限，防止未来实现误复位 owner 期限而无限轮询。
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(
                E_FAIL,
                "Native retirement smoke polling deadline exceeded",
            ));
        }
        let status = state.canvas.borrow_mut().retire_composition();
        has_composition |= status.is_some();
        match status {
            None | Some(Status::Complete) => break,
            Some(Status::Pending) => {
                crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(2))
            }
            Some(status) => {
                return Err(Error::new(
                    E_FAIL,
                    format!("Native retirement smoke failed: {status:?}"),
                ))
            }
        }
    }
    let canvas = state.canvas.borrow();
    if !stamp.matches(&canvas.editor)
        || canvas.paints != paints
        || canvas.confirmed_paints != confirmed
    {
        return Err(Error::new(
            E_FAIL,
            "Retirement changed document or paint confirmation",
        ));
    }
    println!("native-retirement-smoke: dcomp={has_composition} document-preserved=true paints-preserved=true");
    Ok(())
}
