//! 自绘菜单嵌套消息泵；捕获仅属于本次根面板，退出后才交付命令。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
const TIMER: usize = 0x4f4d54;

/// 调用者须先建立应用模态门禁，结束后再核验目标；禁止跨调用借用编辑模型。
/// 不抢占其他窗口前台，也不接管已有拖动捕获。
pub fn track(owner: HWND, tree: Vec<Item>, point: POINT) -> Result<Option<usize>> {
    track_live(owner, tree, point, |_| Ok(()))
}

/// 动态菜单回调只查询本地完成通道；不能阻塞、借用业务模型或直接执行命令。
pub fn track_live(
    owner: HWND,
    tree: Vec<Item>,
    point: POINT,
    update: impl FnMut(&mut Cascade) -> Result<()>,
) -> Result<Option<usize>> {
    let foreground = unsafe { GetForegroundWindow() };
    unsafe {
        if GetAncestor(owner, GA_ROOT) != foreground {
            return Err(Error::new(E_ACCESSDENIED, "Menu owner is not foreground"));
        }
    }
    run_updated(
        owner,
        tree,
        point,
        foreground,
        (|_| Ok(foreground), |_| {}),
        update,
    )
}

/// 仅用于显式托盘菜单手势：激活菜单根面板，不恢复隐藏的业务 owner。
pub fn track_tray(owner: HWND, tree: Vec<Item>, point: POINT) -> Result<Option<usize>> {
    track_tray_live(owner, tree, point, |_| Ok(()))
}

pub fn track_tray_live(
    owner: HWND,
    tree: Vec<Item>,
    point: POINT,
    update: impl FnMut(&mut Cascade) -> Result<()>,
) -> Result<Option<usize>> {
    run_updated(
        owner,
        tree,
        point,
        unsafe { GetForegroundWindow() },
        (
            |root| unsafe {
                if !SetForegroundWindow(root).as_bool() || GetForegroundWindow() != root {
                    return Err(Error::new(E_ACCESSDENIED, "Tray menu activation denied"));
                }
                Ok(root)
            },
            |_| {},
        ),
        update,
    )
}

#[cfg(test)]
fn run(
    owner: HWND,
    tree: Vec<Item>,
    point: POINT,
    foreground: HWND,
    ready: impl FnOnce(HWND),
) -> Result<Option<usize>> {
    run_prepared(owner, tree, point, foreground, |_| Ok(foreground), ready)
}

#[cfg(test)]
fn run_prepared(
    owner: HWND,
    tree: Vec<Item>,
    point: POINT,
    foreground: HWND,
    prepare: impl FnOnce(HWND) -> Result<HWND>,
    ready: impl FnOnce(HWND),
) -> Result<Option<usize>> {
    run_updated(owner, tree, point, foreground, (prepare, ready), |_| Ok(()))
}

fn run_updated(
    owner: HWND,
    tree: Vec<Item>,
    point: POINT,
    foreground: HWND,
    callbacks: (impl FnOnce(HWND) -> Result<HWND>, impl FnOnce(HWND)),
    mut update: impl FnMut(&mut Cascade) -> Result<()>,
) -> Result<Option<usize>> {
    let (prepare, ready) = callbacks;
    if !unsafe { GetCapture() }.is_invalid() {
        return Err(Error::new(E_ABORT, "Another pointer capture is active"));
    }
    let mut menu = Cascade::open(owner, tree, point)?;
    let root = menu.pages[0].panel.handle()?;
    let initial_foreground = foreground;
    let result = (|| {
        let foreground;
        unsafe {
            if GetForegroundWindow() != initial_foreground {
                return Err(Error::new(
                    E_ABORT,
                    "Menu foreground changed during creation",
                ));
            }
            foreground = prepare(root)?;
            if GetForegroundWindow() != foreground {
                return Err(Error::new(E_ABORT, "Menu activation changed"));
            }
            SetCapture(root);
            if GetCapture() != root {
                return Err(Error::new(E_ABORT, "Menu capture was interrupted"));
            }
            // 定时器仅用于生命周期检查，不轮询系统剪贴板；随根 HWND 销毁。
            if SetTimer(Some(root), TIMER, 30, None) == 0 {
                return Err(Error::from_win32());
            }
        }
        ready(root);
        let mut pressed_button = None;
        while !menu.closed() {
            if unsafe { GetCapture() != root || GetForegroundWindow() != foreground } {
                menu.cancel();
                break;
            }
            update(&mut menu)?;
            if menu.navigation.pending.is_some() {
                return Err(Error::new(E_ABORT, "Menu navigation was not consumed"));
            }
            menu.poll()?;
            if menu.closed() {
                break;
            }
            if menu.navigation.pending.is_some() {
                continue;
            }
            let mut message = MSG::default();
            let status = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
            if status == -1 {
                return Err(Error::from_win32());
            }
            if status == 0 {
                unsafe {
                    PostQuitMessage(message.wParam.0 as i32);
                }
                menu.cancel();
                break;
            }
            // GetMessage 期间可派发 sent messages；必须在消费已取出的输入前复核。
            if menu.pages.iter().any(|page| !page.panel.alive())
                || unsafe { GetCapture() != root || GetForegroundWindow() != foreground }
            {
                menu.cancel();
                break;
            }
            match message.message {
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    if matches!(message.wParam.0, 0x12 | 0x79) {
                        menu.cancel();
                    } else {
                        menu.key(message.wParam.0 as u32)?;
                        if !matches!(message.wParam.0, 0x0d | 0x20 | 0x1b | 0x25..=0x28) {
                            unsafe {
                                let _ = TranslateMessage(&message);
                            }
                        }
                    }
                }
                WM_CHAR | WM_SYSCHAR => menu.character(message.wParam.0 as u16)?,
                WM_KEYUP | WM_SYSKEYUP => {}
                WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
                    if pressed_button.is_none() {
                        pressed_button = Some(message.message);
                        menu.pointer(&message)?;
                    }
                }
                WM_LBUTTONUP | WM_RBUTTONUP => {
                    let down = if message.message == WM_LBUTTONUP {
                        WM_LBUTTONDOWN
                    } else {
                        WM_RBUTTONDOWN
                    };
                    if pressed_button == Some(down) {
                        pressed_button = None;
                        menu.pointer(&message)?;
                    }
                }
                WM_MOUSEMOVE | WM_MOUSEWHEEL => menu.pointer(&message)?,
                WM_MBUTTONDOWN | WM_NCLBUTTONDOWN | WM_NCRBUTTONDOWN => menu.cancel(),
                WM_MBUTTONUP => {}
                WM_TIMER if message.hwnd == root && message.wParam.0 == TIMER => {}
                _ => unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                },
            }
        }
        Ok(menu.take_command())
    })();
    // 不能用已被复用的裸 root 数值释放别人的捕获/定时器。
    if menu.pages.first().is_some_and(|page| page.panel.alive()) {
        unsafe {
            let _ = KillTimer(Some(root), TIMER);
            if GetCapture() == root {
                let _ = ReleaseCapture();
            }
        }
    }
    menu.cancel();
    result
}

impl Cascade {
    pub(super) fn pointer(&mut self, message: &MSG) -> Result<()> {
        self.sync_geometry()?;
        if self.closed {
            return Ok(());
        }
        let mut point = POINT {
            x: message.lParam.0 as i16 as i32,
            y: (message.lParam.0 >> 16) as i16 as i32,
        };
        if message.message != WM_MOUSEWHEEL
            && !unsafe { ClientToScreen(message.hwnd, &mut point) }.as_bool()
        {
            self.cancel();
            return Ok(());
        }
        let level = if matches!(message.message, WM_MOUSEMOVE | WM_LBUTTONUP | WM_RBUTTONUP) {
            self.dragging()
        } else {
            None
        }
        .or_else(|| {
            self.pages.iter().rposition(|page| {
                point.x >= page.bounds.left
                    && point.x < page.bounds.right
                    && point.y >= page.bounds.top
                    && point.y < page.bounds.bottom
            })
        });
        if let Some(level) = level {
            let bounds = self.pages[level].bounds;
            let (x, y) = (point.x - bounds.left, point.y - bounds.top);
            let event = match message.message {
                WM_MOUSEMOVE => Event::Move(x, y),
                WM_LBUTTONDOWN | WM_RBUTTONDOWN => Event::Down(x, y),
                WM_LBUTTONUP | WM_RBUTTONUP => Event::Up(x, y),
                _ => Event::Wheel((message.wParam.0 >> 16) as i16),
            };
            self.event(level, event)?;
        } else if matches!(message.message, WM_LBUTTONDOWN | WM_RBUTTONDOWN) {
            self.cancel();
        } else if matches!(message.message, WM_LBUTTONUP | WM_RBUTTONUP) {
            for page in &mut self.pages {
                page.pressed = None;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_menu_modal_tests.rs"]
mod tests;
