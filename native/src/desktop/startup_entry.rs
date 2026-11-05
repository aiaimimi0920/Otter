//! 既有公开启动API、入口参数与初始窗口交接；COM和消息循环仍由desktop/Application管理。
use super::*;
#[path = "shader_startup.rs"]
pub(super) mod shader;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mode {
    Normal,
    Smoke,
    WorkspaceCheck,
}
pub(super) struct Runtime {
    pub(super) settings: settings_startup::Loaded,
    pub(super) shader: shader::Loaded,
    pub(super) workspace: Option<super::workspace_profile::Profile>,
    pub(super) workspace_check: bool,
}
/// 初始窗口到Application的唯一交接；从desktop入口提取，保持安装及失败清理顺序。
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
    let document = document_window::DocumentWindow::create(
        instance,
        state,
        !smoke && !hidden_check && workspace.is_none(),
    )?;
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
    if smoke {
        return startup_smoke::run(instance, &document, image_path, &settings, shader);
    }
    let mut app = application::Application::new(instance, document);
    app.install_capture_history(hidden_check)?;
    app.install_shader(shader.installation);
    app.install_profile_target(settings.default_target);
    if let Some(settings) = settings.source {
        app.install_startup_settings(settings)?;
    }
    app.install_workspace_checkpoint(workspace, workspace_check)?;
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
/// 显式本地启动输入；不保存到 settings/session，不从环境或 Art 中发现 DLL。
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
    pub(super) fn resolve(
        &self,
        smoke: bool,
        check: bool,
    ) -> Result<(settings_startup::Loaded, shader::Loaded)> {
        let shader = shader::resolve(self, smoke)?;
        let settings = settings_startup::resolve(self.settings, smoke || check)?;
        Ok((settings, shader))
    }
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
    /// 隐藏只读启动验证；必须显式提供隔离 profile，不发现真实用户数据。
    pub fn check_workspace(self) -> Result<()> {
        run_source(None, Mode::WorkspaceCheck, false, false, false, None, self)
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
