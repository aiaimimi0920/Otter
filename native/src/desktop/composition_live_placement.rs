//! Live 初始客户区的物理定位；先进入目标显示器再计算非客户区，不改变捕获范围。
use super::*;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};

fn dimensions(rect: RECT) -> Result<D2D_SIZE_U> {
    let width = u32::try_from(i64::from(rect.right) - i64::from(rect.left))
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid Live placement width"))?;
    let height = u32::try_from(i64::from(rect.bottom) - i64::from(rect.top))
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid Live placement height"))?;
    let size = D2D_SIZE_U { width, height };
    super::super::super::validate_size(size)?;
    Ok(size)
}
fn outer(screen: RECT, inset: RECT) -> Result<RECT> {
    let add = |value: i32, delta: i32| {
        value
            .checked_add(delta)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Live placement overflows desktop coordinates"))
    };
    Ok(RECT {
        left: add(screen.left, inset.left)?,
        top: add(screen.top, inset.top)?,
        right: add(screen.left, inset.right)?,
        bottom: add(screen.top, inset.bottom)?,
    })
}
impl LiveWindow {
    /// 调用方须处于 PMv2 上下文；返回前仍隐藏，捕获排除和定位成功后才允许 worker 启动。
    pub fn new_at(consumer: Consumer<Texture>, screen: RECT) -> Result<Self> {
        Self::create_at_with(consumer, screen, |window, size| {
            window.place_content(screen, size)
        })
    }
    fn create_at_with(
        consumer: Consumer<Texture>,
        screen: RECT,
        place: impl FnOnce(&Self, D2D_SIZE_U) -> Result<()>,
    ) -> Result<Self> {
        let size = dimensions(screen)?;
        let window = Self::new(consumer, size.width, size.height)?;
        place(&window, size)?;
        Ok(window)
    }
    fn place_content(&self, screen: RECT, size: D2D_SIZE_U) -> Result<()> {
        let window = self;
        unsafe {
            // 不抢焦点、不改变 Z 顺序；第一次移动用于确定目标 monitor 的 HWND DPI。
            SetWindowPos(
                window.hwnd,
                None,
                screen.left,
                screen.top,
                size.width as i32,
                size.height as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
        }
        place_client(self.hwnd, screen)
    }
}
/// 调整现有客户区；与初始化共用非客户区补偿，不把标题栏尺寸纳入图像缩放比。
pub(super) fn place_client(hwnd: HWND, screen: RECT) -> Result<()> {
    place_client_while(hwnd, screen, || true)
}
/// 每次可重入定位前后复核凭证，旧拖动不能覆盖更新操作或取消结果。
pub(super) fn place_client_while(
    hwnd: HWND,
    screen: RECT,
    current: impl Fn() -> bool,
) -> Result<()> {
    let size = dimensions(screen)?;
    unsafe {
        // 非客户区扩展可能跨越不同 DPI 显示器的接缝；只做有界校正，不形成重定位循环。
        for _ in 0..3 {
            if !current() {
                return Err(Error::new(E_ABORT, "Live placement superseded"));
            }
            let dpi = GetDpiForWindow(hwnd);
            if dpi == 0 {
                return Err(Error::new(E_FAIL, "Live placement DPI unavailable"));
            }
            let mut inset = RECT {
                left: 0,
                top: 0,
                right: size.width as i32,
                bottom: size.height as i32,
            };
            AdjustWindowRectExForDpi(
                &mut inset,
                WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32),
                false,
                WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32),
                dpi,
            )?;
            let frame = outer(screen, inset)?;
            SetWindowPos(
                hwnd,
                None,
                frame.left,
                frame.top,
                frame.right - frame.left,
                frame.bottom - frame.top,
                // 初始精确定位不是用户拖边；避免默认最小跟踪宽度扩大合法的窄选区。
                // 仍接收 WM_WINDOWPOSCHANGED/WM_SIZE，保留正常呈现尺寸失效。
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSENDCHANGING,
            )?;
            if !current() {
                return Err(Error::new(E_ABORT, "Live placement superseded"));
            }
            // 精确验证客户区而非只验证外框，拒绝系统约束导致的静默偏移/缩放。
            let mut origin = POINT::default();
            let mut client = RECT::default();
            GetClientRect(hwnd, &mut client)?;
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return Err(Error::from_win32());
            }
            if origin.x == screen.left
                && origin.y == screen.top
                && client.right == size.width as i32
                && client.bottom == size.height as i32
            {
                return Ok(());
            }
        }
    }
    Err(Error::new(
        E_FAIL,
        "Live client placement does not match requested physical rectangle",
    ))
}

#[cfg(test)]
#[path = "composition_live_placement_tests.rs"]
mod tests;
