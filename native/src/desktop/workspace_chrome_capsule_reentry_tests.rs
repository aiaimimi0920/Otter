//! 同步child消息安装后继时，旧place/focus/close/request栈不得撤销或覆盖它。
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};

struct Replace<'a> {
    ui: &'a Ui,
    port: Port,
    message: u32,
    once: Cell<bool>,
}
unsafe extern "system" fn replace(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let target = &*(data as *const Replace<'_>);
    if msg == target.message && !target.once.replace(true) {
        request(target.ui, target.port, false);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn synchronous_child_replacement_survives_old_focus_place_close_and_request_stacks() {
    unsafe {
        for variant in 0..4 {
            let mut app = app();
            let panel = open(&mut app, "text", variant >= 2);
            let ui = app.chrome.ui.clone();
            let target = Replace {
                ui: &ui,
                port: port(&app, "n"),
                message: match variant {
                    0 => WM_SETFOCUS,
                    1 => WM_WINDOWPOSCHANGED,
                    _ => WM_SHOWWINDOW,
                },
                once: Cell::new(false),
            };
            let hwnd = if variant == 0 {
                edit(&panel)
            } else {
                panel.handle()
            };
            SetWindowSubclass(hwnd, Some(replace), 536, &target as *const _ as usize)
                .ok()
                .unwrap();
            match variant {
                0 => {
                    request(&ui, port(&app, "text"), true);
                    app.sync_workspace_chrome(true);
                }
                1 => {
                    // 同尺寸SetWindowPos不保证发CHANGED；先制造真实child尺寸变化。
                    let open = selected(&ui).unwrap();
                    open.height.set(open.height.get() + 1);
                    window::refresh(&ui).unwrap();
                }
                2 => assert!(key(&app, VK_F6.0, false)),
                _ => request(&ui, port(&app, "enabled"), false),
            }
            assert!(target.once.get(), "variant={variant}");
            let successor = selected(&ui).expect("old stack must preserve replacement");
            assert_eq!(successor.field, "n", "variant={variant}");
            assert!(panel.retired());
            assert!(!IsWindowVisible(panel.handle()).as_bool());
            app.sync_workspace_chrome(true);
            assert!(same(&ui, &successor));
            assert!(successor
                .panel
                .borrow()
                .as_ref()
                .is_some_and(|p| !p.retired()));
            app.shutdown();
        }
    }
}
