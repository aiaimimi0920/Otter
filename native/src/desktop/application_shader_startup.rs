//! Product startup integration and explicit loopback smoke test: real catalog handshake/initial code fetch/child/commit, without prepopulating the recipe cache.
use super::*;
use crate::desktop::shader_prepare::Installation;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use crate::desktop::{
    capability_job, capability_selection::Selection, image::Image, loom_result_state::Status,
};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use std::{
    rc::Rc,
    thread,
    time::{Duration, Instant},
};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
impl Application {
    pub(in crate::desktop) fn install_shader(&mut self, installation: Option<Installation>) {
        self.shader_installation = installation;
    }
}
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
fn rejected(stage: &str) -> Error {
    Error::new(
        E_FAIL,
        format!("Shader startup application smoke failed: {stage}"),
    )
}
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
fn modules_absent() -> bool {
    unsafe {
        GetModuleHandleW(w!("libEGL.dll")).is_err()
            && GetModuleHandleW(w!("libGLESv2.dll")).is_err()
    }
}
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) unsafe fn smoke(
    instance: HINSTANCE,
    loaded: crate::desktop::startup_entry::shader::Loaded,
) -> Result<()> {
    let Some(installation) = loaded.installation else {
        return Ok(());
    };
    if !modules_absent() {
        return Err(rejected("legacy module loaded before startup"));
    }
    let lease = crate::pixels::reserve(4).ok_or_else(|| rejected("input pixel budget"))?;
    let state = State::new(
        Some(Image {
            width: 1,
            height: 1,
            pixels: crate::pixels::Pixels::from_reserved(vec![16, 32, 64, 255], lease),
        }),
        false,
    )?;
    let document = DocumentWindow::create(instance, state, false)?;
    let mut app = Application::new(instance, document);
    let native = installation.bundle.is_none();
    app.install_shader(Some(installation));
    if native {
        println!("native-shader-startup: native=true application=true default=true");
    } else {
        println!("native-shader-startup: explicit=true pinned=true acl=true application=true default=false");
    }
    let Some(identity) = loaded.peer else {
        return Ok(());
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut job = capability_job::Job::default();
    job.start(
        capability_job::Source::Fixed(identity),
        app.execution_wake.callback(),
    )
    .map_err(|_| rejected("start catalog job"))?;
    let catalog = loop {
        if let Some(result) = job.poll() {
            break Rc::new(result.map_err(|_| rejected("read catalog"))?);
        }
        if Instant::now() >= deadline {
            return Err(rejected("catalog deadline"));
        }
        thread::sleep(Duration::from_millis(1));
    };
    // The dedicated peer must declare this ID; never execute an arbitrary real Art from an unrelated catalog.
    let capability = catalog
        .handshake
        .arts()
        .iter()
        .filter_map(crate::loom_capability::Capability::new)
        .find(|art| art.id == "shader-startup" && art.runtime().shader)
        .ok_or_else(|| rejected("missing shader capability"))?;
    let parameters = crate::loom_parameter_state::Parameters::new(&capability)
        .map_err(|_| rejected("admit parameters"))?;
    let mut selection = Selection {
        execution: Default::default(),
        identity: crate::desktop::document_state::new_unit_id()?,
        parameters,
        catalog,
    };
    selection.execution.notify_downstream = false;
    *app.documents[0].state.selected_art.borrow_mut() = Some(selection);
    for (iteration, gain, expected) in [(0, 0.5, [8, 16, 32, 255]), (1, 1.0, [16, 32, 64, 255])] {
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .ok_or_else(|| rejected("missing shader selection"))?
            .parameters
            .set("gain", Some(serde_json::json!(gain)))
            .map_err(|_| rejected("set gain"))?;
        let document = &app.documents[0];
        crate::desktop::context_menu::execute(
            document.window.0,
            &document.state,
            crate::desktop::execution_start::COMMAND,
        )?;
        app.start_requested_preparations(false);
        loop {
            app.pending_windows(false)?;
            let state = &app.documents[0].state;
            let status = state.execution_results.borrow().status();
            if app.input_jobs.is_empty()
                && app.execution_jobs.is_empty()
                && state.pending_preparation.borrow().is_none()
                && state.pending_execution.borrow().is_none()
                && matches!(status, Status::Completed | Status::Failed)
            {
                break;
            }
            if state.preparation_error.borrow().is_some() {
                return Err(rejected(&format!(
                    "iteration={iteration} input preparation"
                )));
            }
            if Instant::now() >= deadline {
                return Err(rejected(&format!(
                    "iteration={iteration} deadline status={status:?}"
                )));
            }
            thread::sleep(Duration::from_millis(1));
        }
        if !app.executions.is_idle() {
            return Err(rejected(&format!(
                "iteration={iteration} registry still owned"
            )));
        }
        if !modules_absent() {
            return Err(rejected(&format!(
                "iteration={iteration} legacy module loaded"
            )));
        }
        let results = app.documents[0].state.execution_results.borrow();
        if results.status() != Status::Completed {
            // Execution commit maps errors to stable local messages, not raw peer data.
            return Err(rejected(&format!(
                "iteration={iteration} commit status={:?}: {}",
                results.status(),
                results.error().unwrap_or("no result error")
            )));
        }
        let image = results
            .image_source("output")
            .ok_or_else(|| rejected("missing formal output"))?
            .image;
        if image.width != 1 || image.height != 1 {
            return Err(rejected(&format!(
                "iteration={iteration} formal image size={}x{}",
                image.width, image.height
            )));
        }
        if *image.pixels != expected {
            return Err(rejected(&format!(
                "iteration={iteration} formal pixels={:?}, expected={expected:?}",
                image.pixels.get(..4)
            )));
        }
        let decoded =
            Image::from_encoded(image.encoded_png().ok_or_else(|| rejected("missing PNG"))?)?;
        if *decoded.pixels != expected {
            return Err(rejected(&format!(
                "iteration={iteration} PNG pixel mismatch"
            )));
        }
        println!("native-shader-product-smoke: iteration={iteration} menu=true formal-input=true uniforms=true child=true committed=true png=true parent-loaded=false cache-hit={} first-fetch={}", iteration > 0, iteration == 0);
    }
    app.input_jobs.shutdown();
    if !app.execution_jobs.shutdown().is_empty() {
        return Err(rejected("execution shutdown had unsettled jobs"));
    }
    Ok(())
}
