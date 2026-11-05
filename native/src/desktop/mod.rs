//! HWND/COM 生命周期限定在此模块；测试入口不注册全局快捷键或读取正式数据。
mod annotation_bezier;
mod annotation_blur;
mod annotation_dash;
mod annotation_freehand;
mod annotation_linear;
mod annotation_mosaic;
mod annotation_opacity;
mod annotation_paintbrush;
mod annotation_polygons;
mod annotation_rotation;
mod annotation_serial;
mod annotation_shapes;
mod annotation_text;
pub use annotation_text::measure_flip_metrics;
mod annotations;
mod app_icon;
pub mod tile_terminal;
mod tile_window;
#[cfg(test)]
pub(crate) fn tile_test_window(
    shared: std::sync::Arc<crate::tile_shared::Shared>,
    output: &crate::tile_outputs::TileOutput,
) -> tile_window::Window {
    tile_window::Window::new(shared, output).unwrap()
}
#[cfg(test)]
pub(crate) fn tile_test_pump() {
    tile_terminal::pump().unwrap();
}
mod application;
mod application_hotkey;
mod art_local_owner;
pub mod art_local_tasks;
mod art_local_window;
mod blur_default;
mod blur_input;
mod blur_numeric;
mod blur_selection;
mod cache_input;
mod canvas;
mod capability_catalog;
pub mod capability_job;
mod capability_parameter_menu;
mod capability_parameters;
mod capability_selection;
pub mod capture;
mod capture_crop;
mod capture_input;
mod capture_magnifier;
mod capture_protection;
mod capture_region;
mod capture_selection;
mod capture_windows;
mod clipboard;
mod clipboard_bitmap;
mod clipboard_dib;
mod clipboard_editable;
mod clipboard_input;
mod clipboard_placement;
mod color_dialog;
pub mod composition;
mod composition_creation;
pub mod composition_d3d12;
#[path = "composition_d3d12_lifetime.rs"]
mod composition_lifetime;
mod composition_present_status;
mod composition_retirement;
#[cfg(test)]
mod composition_software_transition_tests;
mod configuration_stamp;
mod context_menu;
mod copy_job;
mod copy_smoke;
mod crop_apply;
mod crop_effect_smoke;
mod crop_input;
mod crop_smoke;
mod crop_window;
mod device_authorization_ui;
mod dimensions_menu;
pub mod display_composition;
mod display_edit;
mod display_frame;
mod display_snapshot;
mod document_state;
mod document_window;
mod document_window_smoke;
mod drag_input;
pub mod drag_prepare;
pub mod drag_preview;
pub mod drag_transport;
mod edit_input;
#[cfg(test)]
mod edit_input_tests;
mod effect_cache;
mod erase_gesture;
mod erase_input;
mod erase_preview;
mod execution_cancel;
mod execution_configuration;
mod execution_settings;
mod execution_start;
mod export;
mod export_job;
mod export_naming;
mod file_dialog;
mod file_drop;
mod file_drop_smoke;
mod flip_input;
mod flip_smoke;
mod global_capture;
mod group_lock;
mod group_menu;
mod hit_region;
mod host_lifetime;
mod image;
mod link_catalog;
mod link_create;
mod link_menu;
pub mod live_relay_image;
pub mod long_capture_source;
pub mod loom_delivery;
pub mod loom_execution_job;
pub mod loom_execution_prepare;
pub mod loom_execution_session;
mod loom_execution_signal;
pub mod loom_execution_tasks;
mod loom_execution_work;
pub mod loom_image_value;
pub mod loom_inline;
pub mod loom_input_job;
mod loom_input_stamp;
pub mod loom_input_tasks;
pub mod loom_output_batch;
pub mod loom_output_inline;
pub mod loom_output_shared;
pub mod loom_result_state;
mod marquee;
mod marquee_smoke;
mod menu_transition;
mod modal_scope;
mod mosaic_cache;
mod multi_selection;
mod naming_context;
mod naming_import_job;
mod naming_input;
mod naming_save_job;
mod node_menu;
mod numeric_style;
mod open_image;
mod open_job;
mod parameter_action;
mod parameter_dialog;
mod parameter_display_value;
mod parameter_file_input;
mod parameter_file_job;
mod parameter_number;
mod parameter_slider;
mod parameter_value;
mod parameter_view;
mod parameter_view_values;
mod paste_input;
mod path_handles;
mod pin_window;
mod popup_menu;
#[cfg(test)]
mod presentation_readback_test;
mod priority_input;
mod procedure;
mod projection_edit_workspace;
mod rasterize;
mod rasterize_input;
mod render_commands;
mod render_frame;
mod render_surface;
mod render_window_visibility;
mod resize_input;
mod rotation_handle;
mod scale_handle;
mod screen_color;
mod selection;
mod selection_hit;
mod serial_input;
mod session_asset_dialog;
pub mod session_asset_root;
pub mod session_binding;
mod session_blur_resources;
pub mod session_candidate;
mod session_coordinate_dialog;
mod session_end;
pub mod session_image;
pub mod session_image_file;
pub mod session_job;
pub mod session_prepare;
mod session_projection_edit;
mod settings_startup;
pub mod shader_execution_job;
mod shader_fetch_render;
pub mod shader_image;
mod shader_invocation;
mod shader_native_image;
mod shader_native_invocation;
#[cfg(test)]
mod shader_native_route_tests;
pub mod shader_parameters;
mod shader_payload;
pub mod shader_prepare;
mod shader_recipe_cache;
mod shape_dialog;
mod smoke_timing;
pub mod software_surface;
mod startup_smoke;
mod sticker_input;
mod sticker_propagation;
mod style_input;
pub mod surface_authorized_window;
pub(crate) mod surface_capabilities;
mod surface_control_color;
mod surface_control_colors;
#[path = "surface_select_font.rs"]
mod surface_control_font;
pub mod surface_edit_control;
mod surface_edit_geometry;
mod surface_edit_messages;
mod surface_edit_mount;
pub mod surface_edit_session;
pub mod surface_host;
mod surface_scroll_input;
mod surface_scroll_repeat;
pub mod surface_select_control;
pub mod surface_slider_control;
pub mod surface_window;
pub mod surface_windows;
mod text_font;
mod text_input;
mod tool_commands;
mod tool_panel;
mod tool_panel_actions;
mod tool_panel_availability;
mod transform_handle_layout;
mod transform_mode;
mod tray;
mod window_geometry_wake;
mod window_input_event;
mod window_priority;

use canvas::Canvas;
mod startup_entry;
use host_lifetime::{Apartment, Window};
use startup_entry::SessionStartup;
pub use startup_entry::{run, run_capture, run_session, Startup};
use std::{
    cell::{Cell, RefCell},
    path::Path,
};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

mod capture_history_ui;
#[path = "frozen_library_ui.rs"]
mod frozen_library_ui;
#[path = "reference_ui.rs"]
mod reference_ui;
pub(super) struct State {
    chrome: application::chrome::Binding,
    parameters_ui: document_window::parameter_panel::Control,
    capture_history: capture_history_ui::Shared,
    projection_edit: RefCell<Option<projection_edit_workspace::Owner>>,
    screen_color: RefCell<Option<screen_color::Session>>,
    drag: drag_input::Session,
    group_catalog: std::rc::Rc<RefCell<Vec<crate::workspace::Group>>>,
    group_id: RefCell<Option<crate::workspace_id::UnitId>>,
    group_locked: Cell<bool>,
    group_hidden: Cell<bool>,
    group_request: RefCell<Option<group_menu::Request>>,
    unit_id: crate::workspace_id::UnitId,
    execution_results: RefCell<loom_result_state::Results>,
    pending_execution: RefCell<Option<loom_execution_job::Execution>>,
    pending_preparation: RefCell<Option<loom_execution_prepare::Pending>>,
    pending_start: RefCell<Option<execution_start::Request>>,
    preparation_error: RefCell<Option<String>>,
    execution_cancel: RefCell<Option<execution_cancel::Request>>,
    execution_configuration: RefCell<Option<execution_configuration::Binding>>,
    catalog: std::rc::Rc<RefCell<capability_catalog::Catalog>>,
    authorization: std::rc::Rc<RefCell<device_authorization_ui::View>>,
    selected_art: RefCell<Option<capability_selection::Selection>>,
    /// 会话 Art 类型独立于能力可用性；缺失绑定不能退化为普通钉图。
    restored_art: bool,
    // 本地执行的节点身份是数据，不是执行许可；恢复/打开仍须重新授权。
    local_art_id: Option<document_state::local_art::Config>,
    restored_naming_art: naming_context::RestoredArtId,
    parameter_view: RefCell<Option<parameter_view::View>>,
    link_menu: RefCell<Option<std::rc::Rc<link_menu::Snapshot>>>,
    link_request: RefCell<Option<link_menu::Request>>,
    link_catalog: RefCell<Option<std::rc::Rc<link_catalog::Catalog>>>,
    link_creation: RefCell<Option<link_catalog::Creation>>,
    parameter_file_request: RefCell<Option<parameter_file_input::Request>>,
    sticker_input: sticker_input::Control,
    accept_upstream: Cell<bool>,
    resolved_display: RefCell<Option<display_frame::Frame>>,
    pending_adoption: RefCell<Option<display_edit::Pending>>,
    new_window: Cell<bool>,
    art_request: RefCell<Option<node_menu::art_picker::Request>>,
    live_request: RefCell<Option<capture_input::live::Request>>,
    save_session_requested: Cell<bool>,
    save_existing_requested: Cell<bool>,
    workspace_autosave: std::rc::Rc<application::workspace_autosave::Control>,
    workflow_sync_requested: Cell<Option<bool>>,
    open_session_requested: Cell<bool>,
    naming: std::rc::Rc<RefCell<crate::file_naming_settings::Settings>>,
    cache_policy: std::rc::Rc<Cell<crate::cache_policy::Policy>>,
    frozen_library: std::rc::Rc<RefCell<frozen_library_ui::View>>,
    copy_authority: copy_job::Authority,
    editable_clipboard: clipboard_editable::Shared,
    naming_import_requested: Cell<bool>,
    naming_save_requested: Cell<bool>,
    naming_update_requested: Cell<bool>,
    naming_autosave: std::rc::Rc<Cell<bool>>,
    open_session_assets_requested: Cell<bool>,
    exit_requested: std::rc::Rc<Cell<bool>>,
    session_end: std::rc::Rc<session_end::Control>,
    session_end_reason: Cell<bool>,
    new_pin: RefCell<Option<Vec<open_image::PreparedDocument>>>,
    capture_origin: Option<POINT>,
    hotkey_toggle: Cell<bool>,
    hotkey_enabled: std::rc::Rc<Cell<bool>>,
    long_hotkey_enabled: std::rc::Rc<Cell<bool>>,
    live_hotkey_enabled: std::rc::Rc<Cell<bool>>,
    capture_active: std::rc::Rc<Cell<bool>>,
    capture_windows: std::rc::Rc<RefCell<Vec<HWND>>>,
    lifetime: host_lifetime::Registration,
    geometry_wake: window_geometry_wake::Wake,
    priority_watcher: RefCell<Option<window_priority::Watcher>>,
    global_capture: Cell<bool>,
    global_long_capture: Cell<bool>,
    global_live_capture: Cell<bool>,
    tray: tray::Tray,
    capture_protection: Cell<[bool; 3]>,
    capture_protection_requested: Cell<bool>,
    tool_panel: Cell<HWND>,
    tool_panel_tool: Cell<Option<crate::editor::Tool>>,
    tool_panel_style: Cell<Option<crate::stroke_style::StrokeStyle>>,
    tool_panel_enabled: Cell<u64>,
    tool_panel_wheel: Cell<[i64; 2]>,
    copy_job: RefCell<Option<copy_job::Job>>,
    file_dialog: std::rc::Rc<Cell<bool>>,
    modals: modal_scope::Scope,
    canvas: RefCell<Canvas>,
    error: RefCell<Option<Error>>,
    export: RefCell<Option<export_job::ExportJob>>,
    open_job: RefCell<Option<open_job::OpenJob>>,
    blur_job: RefCell<crate::blur_job::Job>,
    erase_job: RefCell<crate::erase_job::Job>,
    resize_job: RefCell<crate::resize_job::Job>,
    flip_job: RefCell<crate::flip_job::Job>,
    crop_job: RefCell<Option<crop_apply::Job>>,
    rasterize_job: RefCell<Option<rasterize_input::Job>>,
    render_retries: Cell<u8>,
    render_active: Cell<bool>,
    render_deferred: Cell<bool>,
    hit_region: RefCell<hit_region::HitRegion>,
    priority_anchor: Cell<isize>,
    priority_policy: RefCell<crate::window_priority::PriorityPolicy>,
}

#[path = "workspace_profile.rs"]
mod workspace_profile;
fn run_source(
    image_path: Option<&Path>,
    mode: startup_entry::Mode,
    pinned: bool,
    capture: bool,
    region: bool,
    session: Option<SessionStartup>,
    startup: Startup<'_>,
) -> Result<()> {
    let smoke = mode == startup_entry::Mode::Smoke;
    let workspace_check = mode == startup_entry::Mode::WorkspaceCheck;
    // 在截图、建窗、热键注册前完成显式配置读取；失败不降级成默认设置。
    let (settings, shader) = startup.resolve(
        smoke,
        workspace_check || session.as_ref().is_some_and(|session| session.check),
    )?;
    let workspace = workspace_profile::resolve(
        !smoke && image_path.is_none() && !capture && session.is_none(),
        workspace_check,
    )
    .map_err(|error| Error::new(E_FAIL, format!("Cannot open workspace profile: {error}")))?;
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
    // DocumentWindow 将 State 固定在堆上；宿主资源总是在 COM apartment 之前析构。
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
        let result = startup_entry::run_window(
            instance.into(),
            state,
            smoke,
            image_path,
            session,
            startup_entry::Runtime {
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
