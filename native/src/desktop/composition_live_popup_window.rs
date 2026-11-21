//! 附属弹窗呈现接缝；镜像属于 B，不移动源窗口，菜单不激活、不夺取源 capture。
use super::*;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
const POPUP_TIMER: usize = 0x4f505550;
impl LiveWindow {
    pub(in crate::desktop) fn popup_hwnd(&self) -> HWND {
        self.hwnd
    }
    pub(in crate::desktop) fn popup_watch(&self) -> Result<()> {
        if unsafe { SetTimer(Some(self.hwnd), POPUP_TIMER, 100, None) } == 0 {
            return Err(Error::from_win32());
        }
        self.enable_popup_focus();
        Ok(())
    }
    pub(in crate::desktop) fn configure_popup(&self, owner: HWND, noactivate: bool) -> Result<()> {
        self.state.popup.set(true);
        self.state.popup_noactivate.set(noactivate);
        self.enable_popup_focus();
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_HWNDPARENT, owner.0 as isize);
            if GetWindow(self.hwnd, GW_OWNER)? != owner {
                return Err(Error::new(E_FAIL, "Live popup mirror owner mismatch"));
            }
            let style = GetWindowLongPtrW(self.hwnd, GWL_STYLE);
            SetWindowLongPtrW(self.hwnd, GWL_STYLE, style & !(WS_SYSMENU.0 as isize));
            if noactivate {
                let style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as isize);
            }
            SetWindowPos(
                self.hwnd,
                // 后绑定 owner 不会把已有 HWND 自动提升到 owner 的置顶层。
                // 只跟随自有 B，不能让普通 B 的菜单无条件盖住其他置顶窗口。
                Some(
                    if GetWindowLongPtrW(owner, GWL_EXSTYLE) & WS_EX_TOPMOST.0 as isize != 0 {
                        HWND_TOPMOST
                    } else {
                        HWND_NOTOPMOST
                    },
                ),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )?;
        }
        Ok(())
    }
    pub(in crate::desktop) fn popup_peers(&self, peers: Vec<HWND>) {
        *self.state.popup_peers.borrow_mut() = peers;
    }
    pub(in crate::desktop) fn reparent_popup(&self, owner: HWND) -> Result<()> {
        if unsafe { GetWindow(self.hwnd, GW_OWNER)? } != owner {
            self.configure_popup(owner, self.state.popup_noactivate.get())?;
        }
        Ok(())
    }
    pub(in crate::desktop) fn popup_visible(&self) -> bool {
        self.is_alive()
            && !self.state.group_hidden.get()
            && unsafe { IsWindowVisible(self.hwnd).as_bool() && !IsIconic(self.hwnd).as_bool() }
    }
    pub(in crate::desktop) fn popup_place(&self, screen: RECT) -> Result<()> {
        let owner = unsafe { GetWindow(self.hwnd, GW_OWNER)? };
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !unsafe {
            GetMonitorInfoW(
                MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
        }
        .as_bool()
        {
            return Err(Error::from_win32());
        }
        placement::place_client(self.hwnd, visible_popup(screen, monitor.rcWork)?)
    }
    pub(in crate::desktop) fn popup_image_rect(&self) -> Option<RECT> {
        let mut client = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut client).ok()?;
        }
        let (_, view) = self
            .state
            .presented
            .get([client.right as u32, client.bottom as u32])?;
        let [left, top, right, bottom] = view.destination();
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(self.hwnd, &mut origin) }.as_bool() {
            return None;
        }
        Some(RECT {
            left: origin.x + left.round() as i32,
            top: origin.y + top.round() as i32,
            right: origin.x + right.round() as i32,
            bottom: origin.y + bottom.round() as i32,
        })
    }
}

// 只限制 B 的呈现位置，不移动 A、不缩放内容。源 dialog 可自行选择真实鼠标
// 附近的位置；按 A-B 偏移映射后不能让整个文本框/取消按钮落到屏幕外。
fn visible_popup(screen: RECT, work: RECT) -> Result<RECT> {
    let axis = |start: i32, end: i32, low: i32, high: i32| -> Result<(i32, i32)> {
        let size = i64::from(end) - i64::from(start);
        if size <= 0 || high <= low {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        let start =
            i64::from(start).clamp(i64::from(low), (i64::from(high) - size).max(i64::from(low)));
        Ok((
            i32::try_from(start).map_err(|_| Error::from_hresult(E_INVALIDARG))?,
            i32::try_from(start + size).map_err(|_| Error::from_hresult(E_INVALIDARG))?,
        ))
    };
    let (left, right) = axis(screen.left, screen.right, work.left, work.right)?;
    let (top, bottom) = axis(screen.top, screen.bottom, work.top, work.bottom)?;
    Ok(RECT {
        left,
        top,
        right,
        bottom,
    })
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    #[test]
    fn modal_mirror_stays_visible_without_cropping_or_resizing() {
        let screen = RECT {
            left: -217,
            top: 17,
            right: 225,
            bottom: 638,
        };
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert_eq!(
            visible_popup(screen, work).unwrap(),
            RECT {
                left: 0,
                top: 17,
                right: 442,
                bottom: 638
            }
        );
        let screen = RECT {
            left: 1780,
            top: 900,
            right: 1965,
            bottom: 1472,
        };
        assert_eq!(
            visible_popup(screen, work).unwrap(),
            RECT {
                left: 1735,
                top: 508,
                right: 1920,
                bottom: 1080
            }
        );
    }
    #[test]
    fn negative_monitor_and_oversized_popup_keep_content_size() {
        let work = RECT {
            left: -1920,
            top: -100,
            right: 0,
            bottom: 900,
        };
        let screen = RECT {
            left: 100,
            top: -200,
            right: 300,
            bottom: 1000,
        };
        assert_eq!(
            visible_popup(screen, work).unwrap(),
            RECT {
                left: -200,
                top: -100,
                right: 0,
                bottom: 1100
            }
        );
        assert!(visible_popup(RECT::default(), work).is_err());
    }
}
