//! 小型原生菜单管理器：配对/批准等待/续签与独立输出进程，不持后台呈现资源。
use super::*;
use std::{cell::Cell, sync::OnceLock};
use windows::{
    core::{w, HRESULT, PCWSTR},
    Win32::System::LibraryLoader::GetModuleHandleW,
};
struct State {
    action: Cell<u32>,
    closed: Cell<bool>,
}
struct Window {
    hwnd: HWND,
    state: Box<State>,
}
impl Window {
    fn take_action(&self) -> Option<u32> {
        // pump 可同时消费命令和关闭；销毁后的待办不再授权启动独立输出。
        (!self.state.closed.get()).then(|| self.state.action.replace(0))
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        if !self.state.closed.get() {
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
pub(super) fn run(origin: &str, profile: Option<&Path>) -> Result<()> {
    let outputs = crate::tile_outputs::enumerate().map_err(fail)?;
    let window = unsafe { create(&outputs)? };
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    let mut owner = Owner::new();
    while !window.state.closed.get() {
        pump()?;
        let Some(action) = window.take_action() else {
            break;
        };
        if !authorization_action(&window, &mut owner, action, origin, profile)? && action >= 100 {
            if let Some(output) = outputs.get((action - 100) as usize) {
                let mut command = std::process::Command::new(
                    std::env::current_exe().map_err(|_| fail("Tile executable unavailable"))?,
                );
                command.args(["--tile-output", &output.output_id, "--loom", origin]);
                if let Some(profile) = profile {
                    command.arg("--profile").arg(profile);
                }
                use std::os::windows::process::CommandExt;
                let result = command.creation_flags(0x08000000).spawn();
                title(
                    &window,
                    if result.is_ok() {
                        "Otter Tile · 输出已启动"
                    } else {
                        "Otter Tile · 输出启动失败"
                    },
                );
            }
        }
        for event in owner.poll() {
            title(
                &window,
                match event {
                    Event::Pairing { .. } => "Otter Tile · 请在 Loom 批准设备",
                    Event::Authorized { .. } => "Otter Tile · 设备已授权",
                    Event::Failed { .. } => "Otter Tile · 设备授权失败",
                    _ => "Otter Tile",
                },
            );
        }
        wait(100);
    }
    owner.close();
    while !owner.finished() {
        pump()?;
        owner.poll();
        wait(25);
    }
    Ok(())
}
// 菜单意图与真实配置/Owner共用一个消费点；关闭后不创建授权任务。
fn authorization_action(
    window: &Window,
    owner: &mut Owner,
    action: u32,
    origin: &str,
    profile: Option<&Path>,
) -> Result<bool> {
    if window.state.closed.get() || !(1..=3).contains(&action) {
        return Ok(false);
    }
    let action = match action {
        1 => Action::RequestPairing,
        2 => Action::AwaitApproval,
        _ => Action::Renew,
    };
    let result = owner.start(Task::Configured {
        config: config(origin, profile)?,
        action,
    });
    title(
        window,
        if result.is_ok() {
            "Otter Tile · 等待设备授权"
        } else {
            "Otter Tile · 授权任务未启动"
        },
    );
    Ok(true)
}
fn title(window: &Window, s: &str) {
    if window.state.closed.get() {
        return;
    }
    let text: Vec<_> = s.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let _ = SetWindowTextW(window.hwnd, PCWSTR(text.as_ptr()));
    }
}
unsafe fn create(outputs: &[crate::tile_outputs::TileOutput]) -> Result<Window> {
    static ONCE: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    ONCE.get_or_init(|| {
        (|| -> Result<()> {
            if RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                lpszClassName: w!("Otter.Native.TileManager"),
                ..Default::default()
            }) == 0
            {
                Err(Error::from_win32())
            } else {
                Ok(())
            }
        })()
        .map_err(|e| e.code())
    })
    .map_err(Error::from_hresult)?;
    let state = Box::new(State {
        action: Cell::new(0),
        closed: Cell::new(false),
    });
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        w!("Otter.Native.TileManager"),
        w!("Otter Tile"),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        760,
        320,
        None,
        None,
        Some(HINSTANCE(GetModuleHandleW(None)?.0)),
        Some((&*state as *const State).cast()),
    )?;
    let window = Window { hwnd, state };
    struct Menu(HMENU, bool);
    impl Drop for Menu {
        fn drop(&mut self) {
            if self.1 {
                unsafe {
                    let _ = DestroyMenu(self.0);
                }
            }
        }
    }
    let mut menu_owner = Menu(CreateMenu()?, true);
    let menu = menu_owner.0;
    for (id, text) in [(1, "请求设备配对"), (2, "等待批准"), (3, "续签授权")] {
        let wide: Vec<_> = text.encode_utf16().chain(Some(0)).collect();
        AppendMenuW(menu, MF_STRING, id, PCWSTR(wide.as_ptr()))?;
    }
    let mut submenu_owner = Menu(CreatePopupMenu()?, true);
    let submenu = submenu_owner.0;
    for (index, output) in outputs.iter().enumerate() {
        let text: Vec<_> = format!("{} · {}×{}", output.name, output.width, output.height)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        AppendMenuW(submenu, MF_STRING, index + 100, PCWSTR(text.as_ptr()))?;
    }
    AppendMenuW(menu, MF_POPUP, submenu.0 as usize, w!("启动输出"))?;
    submenu_owner.1 = false;
    SetMenu(hwnd, Some(menu))?;
    menu_owner.1 = false;
    Ok(window)
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    match msg {
        WM_COMMAND => {
            state.action.set((wp.0 & 0xffff) as u32);
            LRESULT(0)
        }
        WM_DESTROY => {
            state.closed.set(true);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

#[cfg(test)]
#[path = "tile_manager_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tile_native_joint_tests.rs"]
mod joint;
