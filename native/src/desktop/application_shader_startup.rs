//! 产品启动接入及显式回环冒烟：真实目录握手/首次取码/child/commit，不预填配方缓存。
use super::*;
use crate::desktop::{
    capability_job, capability_selection::Selection, image::Image, loom_result_state::Status,
    shader_prepare::Installation,
};
use std::{
    rc::Rc,
    thread,
    time::{Duration, Instant},
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
impl Application {
    pub(in crate::desktop) fn install_shader(&mut self, installation: Option<Installation>) {
        self.shader_installation = installation;
    }
}
fn rejected() -> Error {
    Error::new(E_FAIL, "Shader startup application smoke failed")
}
fn modules_absent() -> bool {
    unsafe {
        GetModuleHandleW(w!("libEGL.dll")).is_err()
            && GetModuleHandleW(w!("libGLESv2.dll")).is_err()
    }
}
pub(super) unsafe fn smoke(
    instance: HINSTANCE,
    loaded: crate::desktop::startup_entry::shader::Loaded,
) -> Result<()> {
    let Some(installation) = loaded.installation else {
        return Ok(());
    };
    if !modules_absent() {
        return Err(rejected());
    }
    let lease = crate::pixels::reserve(4).ok_or_else(rejected)?;
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
    .map_err(|_| rejected())?;
    let catalog = loop {
        if let Some(result) = job.poll() {
            break Rc::new(result.map_err(|_| rejected())?);
        }
        if Instant::now() >= deadline {
            return Err(rejected());
        }
        thread::sleep(Duration::from_millis(1));
    };
    // 专用 peer 必须声明此 ID，不能在无关目录中任选一个真实 Art 执行。
    let capability = catalog
        .handshake
        .arts()
        .iter()
        .filter_map(crate::loom_capability::Capability::new)
        .find(|art| art.id == "shader-startup" && art.runtime().shader)
        .ok_or_else(rejected)?;
    let parameters =
        crate::loom_parameter_state::Parameters::new(&capability).map_err(|_| rejected())?;
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
            .ok_or_else(rejected)?
            .parameters
            .set("gain", Some(serde_json::json!(gain)))
            .map_err(|_| rejected())?;
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
            if state.preparation_error.borrow().is_some() || Instant::now() >= deadline {
                return Err(rejected());
            }
            thread::sleep(Duration::from_millis(1));
        }
        if !app.executions.is_idle() || !modules_absent() {
            return Err(rejected());
        }
        let results = app.documents[0].state.execution_results.borrow();
        if results.status() != Status::Completed {
            return Err(rejected());
        }
        let image = results.image_source("output").ok_or_else(rejected)?.image;
        if *image.pixels != expected || image.width != 1 || image.height != 1 {
            return Err(rejected());
        }
        let decoded = Image::from_encoded(image.encoded_png().ok_or_else(rejected)?)?;
        if *decoded.pixels != expected {
            return Err(rejected());
        }
        println!("native-shader-product-smoke: iteration={iteration} menu=true formal-input=true uniforms=true child=true committed=true png=true parent-loaded=false cache-hit={} first-fetch={}", iteration > 0, iteration == 0);
    }
    app.input_jobs.shutdown();
    if !app.execution_jobs.shutdown().is_empty() {
        return Err(rejected());
    }
    Ok(())
}
