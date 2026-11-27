//! Windows host and session/relay orchestration; re-export lower-level crates to preserve module-path contracts.
pub use otter_core::*;
pub use otter_media::*;
pub use otter_runtime::*;

pub mod app_settings_document;
#[cfg(all(test, windows))]
mod art_cache_fixture;
pub mod art_execution_grant;
pub mod art_issuer_trust;
#[cfg(windows)]
mod art_live_trigger;
pub mod art_local_requests;
mod art_offline_store;
pub mod art_package_cache;
mod art_package_content;
pub mod cache_policy;
#[cfg(windows)]
pub mod capture_dynamic;
#[cfg(windows)]
pub mod capture_file;
#[cfg(windows)]
mod capture_history;
#[cfg(windows)]
pub mod capture_job;
#[cfg(windows)]
pub mod clipboard_cache;
#[cfg(windows)]
pub mod clipboard_history;
#[cfg(windows)]
pub mod desktop;
#[cfg(windows)]
pub mod device_approval;
#[cfg(windows)]
pub mod device_authorization_config;
#[cfg(windows)]
pub mod device_authorization_lease;
#[cfg(windows)]
pub mod device_authorization_owner;
#[cfg(windows)]
pub mod device_authorization_runtime;
#[cfg(windows)]
pub mod device_pairing;
#[cfg(windows)]
pub mod device_session_client;
#[cfg(windows)]
pub mod image_decode;
#[cfg(windows)]
pub mod image_thumbnail;
#[cfg(windows)]
pub mod live_job;
#[cfg(windows)]
pub mod live_relay_catalog_runtime;
#[cfg(windows)]
pub mod live_relay_control_owner;
#[cfg(windows)]
pub mod live_relay_control_runtime;
#[cfg(windows)]
pub mod live_relay_http;
pub mod live_relay_media_endpoint;
#[cfg(windows)]
pub mod live_relay_media_pipeline;
#[cfg(windows)]
pub mod live_relay_media_socket;
#[cfg(windows)]
pub mod live_relay_media_transport;
#[cfg(windows)]
pub mod live_relay_media_worker;
#[cfg(windows)]
pub mod live_relay_source_runtime;
pub mod live_relay_viewer_business;
#[cfg(windows)]
pub mod live_relay_viewer_runtime;
pub mod live_relay_viewer_session;
#[cfg(windows)]
pub mod live_visibility_owner;
pub mod loom_art_control;
pub mod loom_art_exchange;
pub mod loom_base64;
pub mod loom_bridge_client;
pub mod loom_discovery;
pub mod loom_discovery_manifest;
pub mod loom_extension_session;
pub mod loom_handshake_exchange;
#[cfg(windows)]
mod loom_local_connection;
pub mod loom_locale_protocol;
#[cfg(windows)]
pub(crate) mod loom_locale_runtime;
pub mod loom_media_target;
pub mod loom_output_leases;
#[cfg(windows)]
mod loom_remote_connection;
mod loom_selected_connection;
pub mod loom_shortcut_protocol;
#[cfg(windows)]
mod media_websocket;
pub mod minified_transition;
#[cfg(windows)]
pub mod native_menu;
#[cfg(windows)]
pub mod projection_account;
#[cfg(windows)]
pub mod projection_delivery;
#[cfg(windows)]
pub mod projection_delivery_runtime;
pub mod projection_edit;
#[cfg(all(test, windows))]
mod projection_joint_input;
#[cfg(test)]
mod runtime_host_tests;
pub mod session_append;
pub mod session_art;
pub mod session_art_batch;
pub mod session_art_write;
pub mod session_assets;
pub mod session_capture;
#[cfg(windows)]
pub(crate) mod session_capture_original;
pub mod session_coordinates;
pub mod session_crop;
pub mod session_crop_write;
pub mod session_document;
#[cfg(windows)]
pub mod session_file_guard;
#[cfg(windows)]
pub mod session_file_replace;
#[cfg(windows)]
pub mod session_file_write;
pub mod session_library;
pub mod session_minified;
pub mod session_placement;
pub mod session_projection_scope;
pub mod session_source;
pub mod session_vector_write;
pub mod session_window;
pub mod session_window_write;
pub mod session_write;
pub mod settings_source;
#[cfg(windows)]
pub mod settings_update;
#[cfg(windows)]
pub mod shader_angle;
#[cfg(windows)]
pub mod shader_candidate;
#[cfg(windows)]
pub mod shader_process;
#[cfg(all(windows, test))]
pub mod shader_worker;
#[cfg(windows)]
mod shader_worker_wire;
#[cfg(windows)]
#[path = "desktop/software_bitmap.rs"]
pub(crate) mod software_dib;
pub mod surface_action_ack;
pub mod surface_activation;
#[cfg(windows)]
pub mod surface_attach_runtime;
pub mod surface_batch_decision_runtime;
pub mod surface_click_event;
pub mod surface_click_route;
pub mod surface_confirmation;
pub mod surface_convergence;
pub mod surface_decision_runtime;
pub mod surface_embedded_control;
pub mod surface_event_batch_runtime;
pub mod surface_event_record;
pub mod surface_event_runtime;
#[cfg(windows)]
pub mod surface_image_paint;
pub mod surface_image_worker_host;
#[path = "surface_webp_process.rs"]
pub mod surface_image_worker_process;
#[cfg(windows)]
pub mod surface_image_worker_session;
pub mod surface_input_event;
#[cfg(windows)]
pub mod surface_javascript;
#[cfg(windows)]
pub mod surface_javascript_event;
#[cfg(windows)]
mod surface_javascript_wire;
#[cfg(windows)]
pub mod surface_javascript_worker;
#[cfg(windows)]
pub mod surface_lifecycle_owner;
#[cfg(windows)]
pub mod surface_lifecycle_runtime;
#[cfg(windows)]
pub mod surface_live_binding;
#[cfg(windows)]
pub mod surface_native_measure;
mod surface_paint_frame;
pub mod surface_presentation;
pub mod surface_recovery_record;
#[cfg(windows)]
pub mod surface_recovery_runtime;
pub mod surface_recovery_store;
#[cfg(windows)]
pub mod surface_resource_runtime;
pub mod surface_resource_snapshot;
pub mod surface_result_commit;
pub mod surface_result_ports;
mod surface_scroll;
pub mod surface_svg_worker;
#[cfg(windows)]
pub mod surface_text;
#[cfg(windows)]
pub mod surface_text_node;
#[cfg(windows)]
pub mod surface_text_styles;
#[cfg(windows)]
pub mod surface_viewport;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub mod surface_webp_smoke;
pub mod surface_webp_worker;
#[cfg(all(test, windows))]
#[path = "../tests/support/temp_root.rs"]
mod temp_root;
#[cfg(all(test, windows))]
#[path = "../tests/support/process.rs"]
mod test_process;
#[cfg(all(test, windows))]
pub(crate) use temp_root::test_temp_root;
#[cfg(windows)]
mod tile_assets;
#[cfg(windows)]
mod tile_identification;
#[cfg(windows)]
mod tile_image;
#[cfg(windows)]
mod tile_live_packet;
#[cfg(windows)]
mod tile_live_queue;
#[cfg(windows)]
mod tile_live_socket;
#[cfg(windows)]
pub mod tile_runtime;
#[cfg(windows)]
mod tile_shared;
mod tile_surface_action_dispatch;
mod tile_surface_actions;
mod tile_surface_activity;
mod tile_surface_click;
mod tile_surface_edit;
mod tile_surface_edits;
mod tile_surface_focus;
mod tile_surface_geometry;
mod tile_surface_hit;
mod tile_surface_local;
#[cfg(windows)]
mod tile_surface_owner;
mod tile_surface_scrollbar;
mod tile_surface_slider;
mod tile_surface_text;
mod tile_surface_value;
#[cfg(windows)]
mod tile_surface_wire;
#[cfg(windows)]
mod tile_surface_worker;
#[cfg(all(windows, test))]
mod tile_tests;
pub mod ui_text;
pub mod unit_node_settings;
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
    #[ignore = "private HLSL warm session child entry; invoked by bounded parent"]
    fn hlsl_session_worker_entry() {
        std::process::exit(if crate::shader_hlsl::session::run_inherited().is_ok() {
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
