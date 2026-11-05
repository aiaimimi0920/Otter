//! 实际自有离屏HWND与Win32菜单；选择回调注入不操作用户鼠标或焦点。
use super::*;
use crate::tile_surface_value::tests::{fixture, select, switch};
use windows::Win32::System::Com::*;
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
fn window(f: &crate::tile_surface_actions::tests::Fixture) -> Window {
    let mut output = crate::tile_tests::output();
    (output.x, output.y) = (-30000, -30000);
    let w = Window::new(f.shared.clone(), &output).unwrap();
    w.tick().unwrap();
    w.tick().unwrap();
    assert!(w.state.painted.get());
    w
}
fn queued(f: &crate::tile_surface_actions::tests::Fixture) -> bool {
    f.shared
        .lock()
        .actions
        .take(&f.state, Instant::now())
        .is_some()
}
#[test]
fn real_switch_down_up_queues_change_and_select_cancel_same_choice_do_not() {
    let _com = Apartment::new();
    let f = fixture(switch());
    let w = window(&f);
    assert!(w.test_art_click([0, 0]));
    assert!(queued(&f));
    assert_ne!(unsafe { GetForegroundWindow() }, w.hwnd);
    let f = fixture(select());
    let w = window(&f);
    assert!(!test_value(w.hwnd, &w.state, [0, 0], None));
    assert!(!queued(&f));
    assert!(!test_value(w.hwnd, &w.state, [0, 0], Some(0)));
    assert!(!queued(&f));
    assert!(test_value(w.hwnd, &w.state, [0, 0], Some(1)));
    assert!(queued(&f));
}
#[test]
fn select_menu_owns_real_items_checks_source_and_escapes_remote_control_text() {
    let _com = Apartment::new();
    let f = fixture(select());
    let field = crate::tile_surface_value::tests::field(&f);
    let Control::Select(options) = field.control else {
        panic!()
    };
    let m = menu(&options, options.source_index()).unwrap();
    let handle = m.0;
    unsafe {
        assert_eq!(GetMenuItemCount(Some(handle)), 3);
        assert!(GetMenuState(handle, 0, MF_BYPOSITION) & MF_CHECKED.0 != 0);
        let mut buffer = [0u16; 100];
        let n = GetMenuStringW(handle, 1, Some(&mut buffer), MF_BYPOSITION);
        assert_eq!(
            String::from_utf16(&buffer[..n as usize]).unwrap(),
            "B && C 新 选项"
        );
    }
    drop(m);
    assert!(!unsafe { IsMenu(handle) }.as_bool());
}
#[test]
fn changed_paint_focus_loss_and_cancel_mode_retire_the_pressed_field() {
    let _com = Apartment::new();
    for message in [WM_KILLFOCUS, WM_CANCELMODE, WM_CAPTURECHANGED] {
        let f = fixture(switch());
        let w = window(&f);
        down(&w.state, [0, 0]);
        unsafe {
            SendMessageW(w.hwnd, message, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        click(w.hwnd, &w.state, [0, 0]);
        assert!(!queued(&f));
    }
    let f = fixture(switch());
    let w = window(&f);
    down(&w.state, [0, 0]);
    w.state.painted.set(false);
    click(w.hwnd, &w.state, [0, 0]);
    assert!(!queued(&f));
}
#[test]
fn menu_reentry_stop_black_freeze_dpi_or_destroy_cannot_submit_the_old_selection() {
    let _com = Apartment::new();
    for case in 0..5 {
        let f = fixture(select());
        let w = window(&f);
        let prior = field(&w.state, [0, 0]).unwrap();
        finish(w.hwnd, &w.state, prior, [0, 0], |_| {
            match case {
                0 => f.shared.stop(),
                1 => f.shared.lock().black = true,
                2 => f.shared.lock().frozen = true,
                3 => unsafe {
                    SendMessageW(w.hwnd, WM_DPICHANGED, Some(WPARAM(0)), Some(LPARAM(0)));
                },
                _ => unsafe {
                    DestroyWindow(w.hwnd).unwrap();
                },
            }
            Ok(Some(1))
        });
        assert!(!queued(&f));
    }
}
