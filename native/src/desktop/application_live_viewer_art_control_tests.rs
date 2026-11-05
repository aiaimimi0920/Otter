//! Native menu control of an independently viewed Art host; self-owned offscreen HWND only.
use super::*;
#[test]
fn local_art_viewer_menu_acquire_input_release_keeps_viewing() {
    run_host(3, true);
}
#[test]
fn local_art_viewer_focus_loss_releases_native_controller() {
    run_host(4, true);
}
#[test]
fn local_art_viewer_stop_releases_before_reclaiming() {
    run_host(5, true);
}
#[test]
fn local_art_viewer_remote_controller_revocation_keeps_viewing() {
    run_host(6, true);
}
#[test]
fn local_art_viewer_input_failure_never_replays_or_changes_editor() {
    run_host(7, true);
}
#[test]
fn local_art_host_reset_releases_input_disables_reacquire_and_preserves_view() {
    run_host(8, true);
}
