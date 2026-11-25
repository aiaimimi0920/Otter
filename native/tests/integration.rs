//! Aggregated native integration and startup-contract tests eliminate 19 redundant binary links.
#[path = "suites/build_resources.rs"]
mod build_resources;
#[path = "suites/exe_resources.rs"]
mod exe_resources;
#[path = "suites/javascript_worker.rs"]
mod javascript_worker;
#[path = "suites/live_process_runtime.rs"]
mod live_process_runtime;
#[path = "suites/live_recovery_parent.rs"]
mod live_recovery_parent;
#[path = "suites/live_recovery_session.rs"]
mod live_recovery_session;
#[path = "suites/live_window_runtime.rs"]
mod live_window_runtime;
#[path = "suites/live_worker_startup.rs"]
mod live_worker_startup;
#[path = "suites/local_art_runtime.rs"]
mod local_art_runtime;
#[cfg(windows)]
#[path = "support/product.rs"]
mod native_product;
#[path = "suites/process_runner.rs"]
mod process_runner;
#[path = "suites/profile_startup.rs"]
mod profile_startup;
#[path = "suites/session_assets.rs"]
mod session_assets;
#[path = "suites/session_startup.rs"]
mod session_startup;
#[path = "suites/settings_recovery.rs"]
mod settings_recovery;
#[path = "suites/settings_startup.rs"]
mod settings_startup;
#[path = "suites/shader_startup.rs"]
mod shader_startup;
#[path = "suites/startup.rs"]
mod startup;
#[path = "suites/tile_product_runtime.rs"]
mod tile_product_runtime;
#[path = "suites/workspace_runtime.rs"]
mod workspace_runtime;
#[path = "suites/workspace_startup.rs"]
mod workspace_startup;
