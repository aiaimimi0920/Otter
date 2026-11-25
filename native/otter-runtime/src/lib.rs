//! Otter domain models, vector-annotation editing, workspace topology, Loom graph parameters, WGC window targets, and constrained subprocess runtimes.
pub use otter_core::*;
pub use otter_media::*;

pub mod art_surface_views;
pub mod blur_job;
#[cfg(windows)]
pub mod capture_target;
#[cfg(windows)]
pub mod capture_wgc;
pub mod edit_history;
pub mod edit_worker_budget;
pub mod editor;
#[cfg(windows)]
pub mod emergency_watchdog;
pub mod erase_job;
pub mod flip_job;
pub mod freehand;
pub mod graph_execution_config;
pub mod graph_execution_wire;
pub mod graph_image;
pub mod graph_param_coercion;
pub mod graph_params;
pub mod graph_requests;
pub mod group_bounds;
pub mod highlighter_bounds;
#[cfg(windows)]
pub mod live_browser_scroll;
#[cfg(windows)]
pub mod live_hover_binding;
#[cfg(windows)]
pub mod live_native_hooks;
#[cfg(windows)]
pub mod live_popup_layer_request;
#[cfg(windows)]
pub mod live_recovery_client;
#[cfg(windows)]
pub mod live_recovery_runtime;
#[cfg(windows)]
pub mod live_recovery_session;
pub mod live_source_observation;
pub mod live_source_uia;
#[cfg(any(test, feature = "test-support"))]
pub mod live_source_uia_test_window;
#[cfg(windows)]
pub mod live_texture;
#[cfg(windows)]
pub mod live_window_target;
pub mod loom_art_response;
pub mod loom_art_wire;
pub mod loom_capability;
pub mod loom_handshake;
pub mod loom_parameter_state;
pub mod parameter_file;
pub mod resize_job;
pub mod session_parameters;
pub mod session_vectors;
#[cfg(windows)]
pub mod shader_hlsl;
pub mod shape_options;
pub mod sticker_image_params;
pub mod text_annotation;
#[cfg(windows)]
pub mod tile_cli;
#[cfg(windows)]
pub mod tile_clock;
#[cfg(windows)]
pub mod tile_geometry;
#[cfg(windows)]
pub mod tile_outputs;
#[cfg(windows)]
pub mod tile_wire;
pub mod tool_profiles;
pub mod workspace;
pub mod workspace_session;
#[cfg(all(test, windows))]
mod runtime_worker_entries {
    #[test]
    #[ignore = "Only launched with inherited restricted worker handles"]
    fn browser_scroll_worker_entry() {
        crate::live_browser_scroll::process::run_inherited().unwrap();
    }
    #[test]
    #[ignore = "private UIA worker entry, only owned bounded parent invokes"]
    fn uia_observation_worker_entry() {
        std::process::exit(
            if crate::live_source_uia::process::run_inherited().is_ok() {
                0
            } else {
                1
            },
        );
    }
    #[test]
    #[ignore = "private HLSL child entry; invoked by bounded parent"]
    fn hlsl_worker_entry() {
        std::process::exit(if crate::shader_hlsl::process::run_inherited().is_ok() {
            0
        } else {
            1
        });
    }
    #[test]
    #[ignore = "private helper launched only by recovery Client fixtures"]
    fn recovery_client_fixture_entry() {
        crate::live_recovery_client::run_fixture_child();
    }
}
