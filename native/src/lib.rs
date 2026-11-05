//! 原生宿主及可测试模型，不依赖 WebView；标注身份使用 Windows 系统 GUID。
pub mod alpha_region;
pub mod app_settings_document;
pub mod arrow_geometry;
pub mod art_execution_grant;
pub mod art_issuer_trust;
#[cfg(windows)]
mod art_live_trigger;
pub mod art_local_requests;
mod art_offline_store;
mod art_package_cache;
mod art_package_content;
mod art_source_cache;
mod art_source_cache_file;
#[cfg(windows)]
pub mod autostart;
pub mod bezier_arrow;
pub mod bezier_geometry;
pub mod blur;
pub mod blur_image;
pub mod blur_job;
#[cfg(windows)]
mod bounded_worker_process;
pub mod brush_dynamics;
pub mod brush_reduction;
pub mod byte_budget;
pub mod cache_policy;
pub mod capture_color;
#[cfg(windows)]
pub mod capture_display;
#[cfg(windows)]
pub mod capture_dynamic;
#[cfg(windows)]
pub mod capture_file;
#[cfg(windows)]
mod capture_history;
#[cfg(windows)]
pub mod capture_job;
#[cfg(windows)]
pub mod capture_platform;
pub mod capture_png;
#[cfg(windows)]
pub mod capture_readback;
#[cfg(windows)]
pub mod capture_target;
#[cfg(windows)]
pub mod capture_wgc;
#[cfg(windows)]
pub mod clipboard_cache;
#[cfg(windows)]
pub mod clipboard_history;
pub mod crop_geometry;
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
pub mod device_identity;
#[cfg(windows)]
mod device_identity_file;
#[cfg(windows)]
pub mod device_identity_store;
#[cfg(windows)]
pub mod device_key_protection;
#[cfg(windows)]
pub mod device_pairing;
#[cfg(windows)]
pub mod device_request_cancel;
#[cfg(windows)]
pub mod device_session_attempt;
#[cfg(windows)]
pub mod device_session_client;
#[cfg(windows)]
pub mod device_session_proof;
mod edit_history;
pub mod editor;
mod emergency_exit;
#[cfg(windows)]
pub mod emergency_watchdog;
pub mod erase;
pub mod erase_job;
pub mod erase_layers;
pub mod file_naming;
pub mod file_naming_settings;
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
pub mod image_decode;
pub mod image_projection;
#[cfg(windows)]
pub mod image_thumbnail;
pub mod linear_constraint;
pub mod live_button_gesture;
pub mod live_frames;
pub mod live_h264;
pub mod live_input;
#[cfg(windows)]
pub mod live_input_worker;
#[cfg(windows)]
pub mod live_job;
#[cfg(target_os = "windows")]
pub mod live_mf;
pub mod live_reclaim;
#[cfg(windows)]
pub mod live_reclaim_watch;
#[cfg(windows)]
pub mod live_recovery_client;
pub mod live_recovery_handshake;
#[cfg(windows)]
pub mod live_recovery_parent;
#[cfg(windows)]
pub mod live_recovery_pipe;
#[cfg(windows)]
pub mod live_recovery_runtime;
pub mod live_recovery_service;
#[cfg(windows)]
pub mod live_recovery_session;
pub mod live_recovery_wire;
#[cfg(windows)]
pub mod live_relay_catalog_runtime;
#[cfg(windows)]
pub mod live_relay_control_owner;
#[cfg(windows)]
pub mod live_relay_control_runtime;
pub mod live_relay_frame;
#[cfg(windows)]
pub mod live_relay_http;
pub mod live_relay_media_endpoint;
pub mod live_relay_media_liveness;
#[cfg(windows)]
pub mod live_relay_media_pipeline;
#[cfg(windows)]
pub mod live_relay_media_socket;
#[cfg(windows)]
pub mod live_relay_media_transport;
#[cfg(windows)]
pub mod live_relay_media_worker;
pub mod live_relay_observation;
pub mod live_relay_pixels;
pub mod live_relay_receive;
#[cfg(windows)]
pub mod live_relay_source_runtime;
pub mod live_relay_trigger_audit;
mod live_relay_trigger_command;
pub mod live_relay_trigger_registration;
pub mod live_relay_video_chain;
#[cfg(windows)]
pub mod live_relay_video_receiver;
pub mod live_relay_viewer_business;
pub mod live_relay_viewer_frames;
pub mod live_relay_viewer_identity;
#[cfg(windows)]
pub mod live_relay_viewer_runtime;
pub mod live_relay_viewer_session;
mod live_relay_viewer_trigger_queue;
mod live_source_observation;
pub mod live_source_uia;
#[cfg(test)]
mod live_source_uia_test_window;
#[cfg(windows)]
pub mod live_texture;
pub mod live_viewport;
#[cfg(windows)]
pub mod live_visibility_owner;
#[cfg(windows)]
pub mod live_window_target;
pub mod long_capture;
#[cfg(windows)]
pub mod loom_address_resolver;
pub mod loom_art_control;
pub mod loom_art_exchange;
pub mod loom_art_response;
pub mod loom_art_wire;
pub mod loom_base64;
pub mod loom_bridge_client;
pub mod loom_bridge_identity;
pub mod loom_bridge_stream;
pub mod loom_capability;
pub mod loom_discovery;
pub mod loom_discovery_manifest;
pub mod loom_handshake;
pub mod loom_handshake_exchange;
#[cfg(windows)]
mod loom_http_transport;
pub mod loom_media_target;
pub mod loom_output_leases;
pub mod loom_parameter_state;
pub mod loom_tls_trust;
#[cfg(windows)]
mod media_websocket;
pub mod message_clock;
pub mod minified_transition;
pub mod mosaic;
#[cfg(windows)]
pub mod native_menu;
pub mod paintbrush_samples;
pub mod parameter_file;
pub mod pin_geometry;
pub mod pixels;
pub mod polygon_geometry;
#[cfg(windows)]
mod profile_directory;
#[cfg(windows)]
pub mod projection_delivery;
#[cfg(windows)]
pub mod projection_delivery_runtime;
pub mod projection_edit;
pub mod projection_receive_policy;
pub mod raster_flip;
mod raster_flip_resize;
pub mod raster_projection;
pub mod raster_source;
mod raster_source_frame;
pub mod resize_job;
pub mod rotation;
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
pub mod session_file_recovery;
#[cfg(windows)]
mod session_file_rename;
#[cfg(windows)]
pub mod session_file_replace;
#[cfg(windows)]
mod session_file_trace;
#[cfg(windows)]
pub mod session_file_write;
mod session_json_fields;
pub mod session_library;
pub mod session_minified;
pub mod session_parameters;
pub mod session_placement;
pub mod session_projection_scope;
pub mod session_source;
pub mod session_vector_write;
pub mod session_vectors;
pub mod session_window;
pub mod session_window_write;
pub mod session_write;
pub mod settings_profile;
pub mod settings_source;
#[cfg(windows)]
pub mod settings_update;
#[cfg(windows)]
pub mod shader_angle;
#[cfg(windows)]
pub mod shader_candidate;
#[cfg(windows)]
pub mod shader_hlsl;
#[cfg(windows)]
pub mod shader_process;
#[cfg(all(windows, test))]
pub mod shader_worker;
#[cfg(windows)]
mod shader_worker_wire;
pub mod shape_constraint;
pub mod shape_options;
pub mod shape_value;
mod sticker_image_params;
pub mod stroke_style;
#[cfg(windows)]
mod tile_assets;
#[cfg(windows)]
pub mod tile_cli;
#[cfg(windows)]
mod tile_clock;
#[cfg(windows)]
mod tile_geometry;
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
#[path = "../../src-tauri/src/tile_outputs.rs"]
pub mod tile_outputs;
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
#[cfg(windows)]
pub mod tile_wire;
// 保留既有源码路径；共享的线程绑定 DIB 只编译为一个资源 owner。
#[cfg(windows)]
#[path = "desktop/software_bitmap.rs"]
pub(crate) mod software_dib;
pub mod surface_accessibility;
pub mod surface_action_ack;
pub mod surface_activation;
#[cfg(windows)]
pub mod surface_attach_runtime;
mod surface_background;
pub mod surface_batch_decision_runtime;
mod surface_border;
mod surface_box;
mod surface_button;
mod surface_button_visual;
pub mod surface_click_event;
pub mod surface_click_route;
pub mod surface_clip;
pub mod surface_color;
pub mod surface_confirmation;
pub mod surface_container;
mod surface_control_colors;
pub mod surface_convergence;
pub mod surface_cross_axis;
pub mod surface_decision_runtime;
pub mod surface_edit_contract;
mod surface_embedded_control;
pub mod surface_event_batch_runtime;
pub mod surface_event_record;
pub mod surface_event_runtime;
pub mod surface_flow;
mod surface_focus;
mod surface_focus_draw;
pub mod surface_form_values;
pub mod surface_geometry;
mod surface_grid;
mod surface_hit_test;
pub mod surface_host_capabilities;
pub(crate) mod surface_image_format;
#[cfg(windows)]
pub mod surface_image_paint;
mod surface_image_worker_host;
#[path = "surface_webp_process.rs"]
mod surface_image_worker_process;
mod surface_image_worker_wire;
pub mod surface_input_draft;
pub mod surface_input_event;
#[cfg(windows)]
pub mod surface_javascript;
#[cfg(windows)]
mod surface_javascript_event;
#[cfg(windows)]
mod surface_javascript_wire;
#[cfg(windows)]
pub mod surface_javascript_worker;
pub mod surface_layout_tree;
mod surface_layout_value;
pub mod surface_length;
#[cfg(windows)]
pub mod surface_lifecycle_owner;
#[cfg(windows)]
pub mod surface_lifecycle_runtime;
#[cfg(windows)]
pub mod surface_live_binding;
#[cfg(windows)]
pub mod surface_native_measure;
mod surface_node_defaults;
pub mod surface_number;
mod surface_overflow;
mod surface_paint_frame;
pub mod surface_presentation;
mod surface_progress;
pub mod surface_recovery_record;
#[cfg(windows)]
pub mod surface_recovery_runtime;
pub mod surface_recovery_store;
#[cfg(windows)]
pub mod surface_resource_runtime;
pub mod surface_resource_snapshot;
pub mod surface_result_commit;
pub mod surface_result_ports;
mod surface_root_box;
mod surface_round_clip;
pub mod surface_scene;
pub mod surface_select;
pub mod surface_slider;
pub mod surface_stream;
pub mod surface_svg_worker;
mod surface_switch;
#[cfg(windows)]
pub mod surface_text;
#[cfg(windows)]
pub mod surface_text_node;
#[cfg(windows)]
pub mod surface_text_styles;
mod surface_textarea;
#[cfg(windows)]
pub mod surface_viewport;
pub(crate) mod surface_webp_header;
pub mod surface_webp_smoke;
pub mod surface_webp_worker;
#[cfg(all(test, windows))]
#[path = "../tests/support/process.rs"]
mod test_process;
pub mod text_annotation;
pub mod tool_profiles;
pub mod transform_gesture;
pub mod viewport;
pub mod window_priority;
pub mod workspace;
pub mod workspace_id;
pub mod workspace_session;

#[cfg(windows)]
pub mod desktop;
mod surface_scroll;
