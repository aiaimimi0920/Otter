//! Route timer messages for host background tasks and short settings feedback; each responsible module still owns its timer lifecycle.
use crate::desktop::{self, State};
use windows::Win32::Foundation::HWND;

pub(super) unsafe fn complete(hwnd: HWND, state: &State, id: usize) -> bool {
    match id {
        desktop::pin_settings::TEXT_TIMER => desktop::pin_settings::text_timer(hwnd, state),
        super::render_recovery::TIMER => super::render_recovery::poll(hwnd, state),
        desktop::export_job::EXPORT_TIMER => desktop::export_job::finish(hwnd, state),
        desktop::open_job::OPEN_TIMER => desktop::open_job::finish(hwnd, state, false),
        desktop::blur_input::TIMER => desktop::blur_input::poll(hwnd, state),
        desktop::erase_input::TIMER => desktop::erase_input::poll(hwnd, state),
        desktop::resize_input::TIMER => desktop::resize_input::poll(hwnd, state),
        desktop::flip_input::TIMER => desktop::flip_input::poll(hwnd, state),
        desktop::crop_apply::TIMER => desktop::crop_apply::poll(hwnd, state),
        desktop::rasterize_input::TIMER => desktop::rasterize_input::poll(hwnd, state),
        _ => return false,
    }
    true
}
