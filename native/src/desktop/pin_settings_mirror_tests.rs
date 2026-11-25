//! Real BUTTON -> asynchronous mirror -> timer delivery -> panel restoration; no input sent to the user's desktop.
use super::*;
use crate::{
    desktop::{pin_settings as settings, tool_panel},
    editor::MirrorState,
};

use super::settings::field;
use std::cell::Cell;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

struct Watch {
    handles: Vec<HWND>,
    enabled: Box<Cell<usize>>,
}
impl Watch {
    unsafe fn new(roots: &[HWND]) -> Self {
        unsafe extern "system" fn child(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
            // Undo/redo eligibility changes with actual history; this is not temporary busy-state flicker from mirroring.
            if !matches!(GetDlgCtrlID(hwnd), 0x1007 | 0x1008) {
                (&mut *(lp.0 as *mut Vec<HWND>)).push(hwnd);
            }
            windows::core::BOOL::from(true)
        }
        let mut handles = roots.to_vec();
        for root in roots {
            let _ = EnumChildWindows(
                Some(*root),
                Some(child),
                LPARAM(&mut handles as *mut _ as isize),
            );
        }
        let watch = Self {
            handles,
            enabled: Box::new(Cell::new(0)),
        };
        for hwnd in &watch.handles {
            SetWindowSubclass(
                *hwnd,
                Some(observe),
                609,
                &*watch.enabled as *const _ as usize,
            )
            .ok()
            .unwrap();
        }
        watch
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        for hwnd in &self.handles {
            unsafe {
                let _ = RemoveWindowSubclass(*hwnd, Some(observe), 609);
            }
        }
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_ENABLE {
        let count = &*(data as *const Cell<usize>);
        count.set(count.get() + 1);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn mirror_buttons_preserve_control_states_and_follow_single_step_undo_redo() {
    unsafe {
        let f = Fixture::new();
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        execute(f.owner.0, &f.state, catalog::BRUSH as usize, false);
        execute(f.owner.0, &f.state, catalog::SETTINGS as usize, false);
        let window = settings::window(&f.state).unwrap();
        let properties = neuro_mini_ui::property_window(f.panel.0).unwrap();
        for (id, expected) in [
            (settings::FLIP_X, MirrorState { x: true, y: false }),
            (settings::FLIP_Y, MirrorState { x: true, y: true }),
            (settings::FLIP_X, MirrorState { x: false, y: true }),
        ] {
            let watch = Watch::new(&[f.panel.0, window, properties]);
            let before = f.state.canvas.borrow().editor.mirror_state();
            SendMessageW(field(window, id), BM_CLICK, None, None);
            let mut dispatched = false;
            for _ in 0..32 {
                let mut msg = MSG::default();
                if !PeekMessageW(
                    &mut msg,
                    Some(f.owner.0),
                    super::super::super::EXECUTE,
                    super::super::super::EXECUTE,
                    PM_REMOVE,
                )
                .as_bool()
                {
                    break;
                }
                let setting = msg.lParam.0 == neuro_mini_ui::PANEL;
                DispatchMessageW(&msg);
                if setting {
                    dispatched = true;
                    break;
                }
            }
            assert!(dispatched);
            assert_eq!(f.state.canvas.borrow().editor.mirror_state(), before);
            assert!(f.state.flip_job.borrow().is_busy());
            // Presentation retains an available appearance, but the actual button execution path still rejects conflicting operations.
            execute(f.owner.0, &f.state, catalog::SHAPE as usize, false);
            assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Brush);
            for _ in 0..8 {
                tool_panel::refresh(f.owner.0);
            }
            assert_eq!(watch.enabled.get(), 0, "mirror busy state flashed controls");
            // Repeated commands during asynchronous single-flight work must neither queue nor switch pressed early.
            let revision = f.state.canvas.borrow().editor.revision();
            crate::desktop::flip_input::start(
                f.owner.0,
                &f.state,
                crate::desktop::flip_input::VERTICAL,
            );
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while f.state.flip_job.borrow().is_busy() {
                assert!(std::time::Instant::now() < deadline);
                SendMessageW(
                    f.owner.0,
                    WM_TIMER,
                    Some(WPARAM(crate::desktop::flip_input::TIMER)),
                    None,
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert_eq!(f.state.canvas.borrow().editor.revision(), revision + 1);
            assert_eq!(f.state.canvas.borrow().editor.mirror_state(), expected);
            assert!(IsWindowEnabled(field(window, id)).as_bool());
            assert_eq!(
                watch.enabled.get(),
                0,
                "mirror completion toggled unrelated controls"
            );
            assert_eq!(neuro_mini_ui::property_window(f.panel.0), Some(properties));
            assert_eq!(settings::window(&f.state), Some(window));
            drop(watch);
            crate::desktop::context_menu::execute(
                f.owner.0,
                &f.state,
                crate::desktop::context_menu::UNDO,
            )
            .unwrap();
            assert_eq!(f.state.canvas.borrow().editor.mirror_state(), before);
            crate::desktop::context_menu::execute(
                f.owner.0,
                &f.state,
                crate::desktop::context_menu::REDO,
            )
            .unwrap();
            assert_eq!(f.state.canvas.borrow().editor.mirror_state(), expected);
        }
        tool_panel::refresh(f.owner.0);
        assert!(settings::is_open(&f.state));
        assert!(f.state.error.borrow().is_none());
    }
}
