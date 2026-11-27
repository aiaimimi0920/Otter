//! Existing public startup APIs, entry arguments, and initial-window handoff; desktop/Application still manage COM and the message loop.
use super::*;
#[path = "shader_startup.rs"]
pub(super) mod shader;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mode {
    Normal,
    Smoke,
    WorkspaceCheck,
    RestartCheck(super::workspace_profile::restart::Expected),
}
pub(super) struct Runtime {
    pub(super) settings: settings_startup::Loaded,
    pub(super) shader: shader::Loaded,
    pub(super) workspace: Option<super::workspace_profile::Profile>,
    pub(super) workspace_check: bool,
}
/// The sole initial-window handoff to Application, extracted from desktop entry while preserving installation and failure-cleanup order.
pub(super) unsafe fn run_window(
    instance: HINSTANCE,
    state: State,
    smoke: bool,
    image_path: Option<&Path>,
    session: Option<SessionStartup>,
    runtime: Runtime,
) -> Result<()> {
    let Runtime {
        settings,
        shader,
        workspace,
        workspace_check,
    } = runtime;
    let hidden_check = workspace_check || session.as_ref().is_some_and(|session| session.check);
    // Always create tray/hotkey hosts hidden; show real content only after restoration, avoiding empty-panel flashes.
    let document = document_window::DocumentWindow::create(instance, state, false)?;
    #[cfg(any(debug_assertions, feature = "smoke-test"))]
    let document = if smoke {
        document.smoke_relocation()?
    } else {
        document
    };
    if let Some(source) = &settings.source {
        settings_startup::announce_recovery(document.window.0, source, smoke || hidden_check);
    }
    if !smoke && !hidden_check {
        document.observe_priority()?;
    }
    #[cfg(any(debug_assertions, feature = "smoke-test"))]
    if smoke {
        return startup_smoke::run(instance, &document, image_path, &settings, shader);
    }
    #[cfg(not(any(debug_assertions, feature = "smoke-test")))]
    let _ = image_path;
    let mut app = application::Application::new(instance, document);
    app.bind_archive_profile(
        workspace
            .as_ref()
            .and_then(|p| p.path.parent())
            .or_else(|| settings.default_target.as_deref().and_then(Path::parent)),
    );
    if !hidden_check {
        app.install_capture_history()?;
    }
    app.install_shader(shader.installation);
    app.install_profile_target(settings.default_target);
    if let Some(settings) = settings.source {
        app.install_startup_settings(settings)?;
    }
    let restart_check = workspace.as_ref().is_some_and(|p| p.claimed.is_some());
    app.install_workspace_checkpoint(workspace, workspace_check)?;
    if restart_check {
        return app.verify_hidden_restore();
    }
    if workspace_check {
        return app.check_restored_startup();
    }
    if session.as_ref().is_some_and(|session| session.check) {
        let session = session.unwrap();
        app.request_restore_input(
            session_job::Input::File(session.path),
            session.root,
            session.space,
        )?;
        return app.check_restored_startup();
    }
    app.run(session)
}
/// Explicit local startup inputs; not saved to settings/session, and no DLL discovery from the environment or Art.
#[derive(Default)]
pub struct Startup<'a> {
    pub settings: Option<&'a Path>,
    pub shader_candidate: Option<&'a Path>,
    pub shader_smoke_peer: Option<&'a Path>,
}
pub fn run(
    image_path: Option<&Path>,
    smoke: bool,
    pinned: bool,
    settings: Option<&Path>,
) -> Result<()> {
    Startup {
        settings,
        ..Default::default()
    }
    .run(image_path, smoke, pinned)
}

pub fn run_capture(pinned: bool, region: bool, settings: Option<&Path>) -> Result<()> {
    Startup {
        settings,
        ..Default::default()
    }
    .run_capture(pinned, region)
}

pub(super) struct SessionStartup {
    pub(super) path: std::path::PathBuf,
    pub(super) root: Option<session_asset_root::Root>,
    pub(super) space: Option<crate::session_placement::Space>,
    pub(super) check: bool,
}
pub fn run_session(
    path: std::path::PathBuf,
    assets: Option<std::path::PathBuf>,
    space: Option<crate::session_placement::Space>,
    check: bool,
    settings: Option<&Path>,
) -> Result<()> {
    Startup {
        settings,
        ..Default::default()
    }
    .run_session(path, assets, space, check)
}
impl Startup<'_> {
    pub fn run(self, image: Option<&Path>, smoke: bool, pinned: bool) -> Result<()> {
        run_source(
            image,
            if smoke { Mode::Smoke } else { Mode::Normal },
            pinned,
            false,
            false,
            None,
            self,
        )
    }
    /// Hidden read-only startup verification; an isolated profile is mandatory and explicit, with no actual-user-data discovery.
    pub fn check_workspace(self) -> Result<()> {
        run_source(None, Mode::WorkspaceCheck, false, false, false, None, self)
    }
    /// Verify a particular published checkpoint with a hidden restore. A
    /// caller-supplied digest is a consistency check, not authorization.
    pub fn check_restart_checkpoint(self, revision: &str, sha256: &str) -> Result<()> {
        let expected = workspace_profile::restart::Expected::parse(revision, sha256)
            .map_err(|e| Error::new(E_INVALIDARG, e.to_string()))?;
        run_source(
            None,
            Mode::RestartCheck(expected),
            false,
            false,
            false,
            None,
            self,
        )
    }
    pub fn run_capture(self, pinned: bool, region: bool) -> Result<()> {
        run_source(None, Mode::Normal, pinned, true, region, None, self)
    }
    pub fn run_session(
        self,
        path: std::path::PathBuf,
        assets: Option<std::path::PathBuf>,
        space: Option<crate::session_placement::Space>,
        check: bool,
    ) -> Result<()> {
        let root = assets
            .as_deref()
            .map(session_asset_root::Root::open)
            .transpose()?;
        run_source(
            None,
            Mode::Normal,
            false,
            false,
            false,
            Some(SessionStartup {
                path,
                root,
                space,
                check,
            }),
            self,
        )
    }
}

fn run_source(
    image_path: Option<&Path>,
    mode: Mode,
    pinned: bool,
    capture: bool,
    region: bool,
    session: Option<SessionStartup>,
    startup: Startup<'_>,
) -> Result<()> {
    let smoke = mode == Mode::Smoke;
    #[cfg(not(any(debug_assertions, feature = "smoke-test")))]
    if smoke {
        return Err(Error::new(
            E_INVALIDARG,
            "--smoke-test is disabled in release builds",
        ));
    }
    let workspace_check = matches!(mode, Mode::WorkspaceCheck | Mode::RestartCheck(_));
    let shader = shader::resolve(&startup, smoke)?;
    // A competing startup must not read policy from the outgoing writer before
    // acquiring its lease. Keep using the pinned root even if discovery changes.
    let workspace = if let Mode::RestartCheck(expected) = mode {
        let root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR").ok_or_else(|| {
            Error::new(
                E_INVALIDARG,
                "--check-restart-checkpoint requires OTTER_NATIVE_PROFILE_DIR",
            )
        })?;
        workspace_profile::Profile::claim(Path::new(&root), expected).map(Some)
    } else {
        workspace_profile::resolve(
            !smoke && image_path.is_none() && !capture && session.is_none(),
            workspace_check,
        )
    }
    .map_err(|error| Error::new(E_FAIL, format!("Cannot open workspace profile: {error}")))?;
    let settings = settings_startup::resolve_in_profile(
        startup.settings,
        smoke || workspace_check || session.as_ref().is_some_and(|session| session.check),
        workspace.as_ref().and_then(|profile| profile.path.parent()),
    )?;
    settings.install_language()?;
    #[cfg(any(debug_assertions, feature = "smoke-test"))]
    if smoke {
        crate::surface_webp_smoke::run()?;
    }
    let _apartment = Apartment::new()?;
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    }
    let image = if capture {
        let windows = if region {
            capture_windows::snapshot()?
        } else {
            Vec::new()
        };
        let frame = capture::virtual_desktop()?;
        if region {
            let Some(image) = capture_region::select(frame.image, frame.origin, windows)? else {
                return Ok(());
            };
            Some(image)
        } else {
            Some(frame.image)
        }
    } else {
        image_path.map(image::Image::open).transpose()?
    };
    let state = State::new(image, pinned)?;
    capture_history_ui::record_initial(&state, capture);
    settings.install(&state)?;
    // DocumentWindow pins State on the heap; host resources always destruct before the COM apartment.
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            style: CS_DBLCLKS,
            lpfnWndProc: Some(procedure::window_proc),
            hInstance: instance.into(),
            lpszClassName: w!("Otter.Native.Workspace"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(Error::from_win32());
        }
        let result = run_window(
            instance.into(),
            state,
            smoke,
            image_path,
            session,
            Runtime {
                settings,
                shader,
                workspace,
                workspace_check,
            },
        );
        let _ = UnregisterClassW(class.lpszClassName, Some(instance.into()));
        result
    }
}
