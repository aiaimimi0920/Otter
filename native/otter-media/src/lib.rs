//! Otter image capture, Surface layout/drawing, and foundational Live runtime.
pub use otter_core::*;

pub mod art_source_cache;
pub mod art_source_cache_file;
#[cfg(windows)]
pub mod autostart;
#[cfg(windows)]
pub mod bounded_worker_process;
pub mod capture_color;
#[cfg(windows)]
pub mod capture_display;
#[cfg(windows)]
pub mod capture_platform;
pub mod capture_png;
#[cfg(windows)]
pub mod capture_readback;
#[cfg(windows)]
pub mod device_identity;
#[cfg(windows)]
pub mod device_identity_file;
#[cfg(windows)]
pub mod device_identity_store;
#[cfg(windows)]
pub mod device_key_protection;
#[cfg(windows)]
pub mod device_request_cancel;
#[cfg(windows)]
pub mod device_session_attempt;
#[cfg(windows)]
pub mod device_session_proof;
#[cfg(windows)]
pub mod elevation_status;
pub mod extension_visual;
pub mod file_naming;
pub mod file_naming_settings;
#[cfg(windows)]
pub mod install_file_leases;
pub mod live_button_gesture;
pub mod live_frames;
pub mod live_h264;
#[cfg(windows)]
pub mod live_hover_owner;
pub mod live_input;
#[cfg(windows)]
pub mod live_input_worker;
#[cfg(target_os = "windows")]
pub mod live_mf;
pub mod live_reclaim;
#[cfg(windows)]
pub mod live_reclaim_watch;
pub mod live_recovery_handshake;
#[cfg(windows)]
pub mod live_recovery_parent;
#[cfg(windows)]
pub mod live_recovery_pipe;
pub mod live_recovery_service;
pub mod live_recovery_wire;
pub mod live_relay_frame;
pub mod live_relay_media_liveness;
pub mod live_relay_observation;
pub mod live_relay_pixels;
pub mod live_relay_receive;
pub mod live_relay_trigger_audit;
pub mod live_relay_trigger_command;
pub mod live_relay_trigger_registration;
pub mod live_relay_video_chain;
#[cfg(windows)]
pub mod live_relay_video_receiver;
pub mod live_relay_viewer_frames;
pub mod live_relay_viewer_identity;
pub mod live_relay_viewer_trigger_queue;
pub mod live_viewport;
#[cfg(windows)]
pub mod loom_address_resolver;
pub mod loom_bridge_identity;
pub mod loom_bridge_stream;
pub mod loom_extension_snapshot;
#[cfg(windows)]
pub mod loom_http_transport;
pub mod loom_tls_trust;
pub mod pin_frame_style;
#[cfg(windows)]
pub mod profile_directory;
#[cfg(windows)]
pub mod session_file_recovery;
#[cfg(windows)]
pub mod session_file_rename;
#[cfg(windows)]
pub mod session_file_trace;
pub mod sticker_extra_text;
pub mod surface_accessibility;
pub mod surface_background;
pub mod surface_border;
pub mod surface_box;
pub mod surface_button;
pub mod surface_button_visual;
pub mod surface_clip;
pub mod surface_color;
pub mod surface_container;
pub mod surface_control_colors;
pub mod surface_cross_axis;
pub mod surface_edit_contract;
pub mod surface_flow;
pub mod surface_focus;
pub mod surface_focus_draw;
pub mod surface_form_values;
pub mod surface_geometry;
pub mod surface_grid;
pub mod surface_hit_test;
pub mod surface_host_capabilities;
pub mod surface_image_format;
pub mod surface_image_worker_wire;
pub mod surface_input_draft;
pub mod surface_layout_tree;
pub mod surface_layout_value;
pub mod surface_length;
pub mod surface_node_defaults;
pub mod surface_number;
pub mod surface_overflow;
pub mod surface_progress;
pub mod surface_root_box;
pub mod surface_round_clip;
pub mod surface_scene;
pub mod surface_select;
pub mod surface_slider;
pub mod surface_stream;
pub mod surface_switch;
pub mod surface_textarea;
pub mod surface_webp_header;
pub mod unit_attachment;
pub mod unit_attachment_store;
