//! The preview HWND forwards only to a verified host; clicking ends the preview first to avoid a second drag/edit state.
use super::*;
use std::sync::OnceLock;
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};
const CLASS: windows::core::PCWSTR = w!("Otter.Native.ZoomPreview");
#[derive(Clone, Copy)]
struct Route {
    owner: HWND,
    marker: isize,
}
pub(super) struct Window {
    pub hwnd: HWND,
    _route: Box<Route>,
    binding: Option<crate::desktop::application::chrome::Binding>,
}
impl Window {
    pub fn shared(&self) -> bool {
        self.binding.is_some()
    }
    pub unsafe fn sync_order(&self, owner: HWND) -> Result<()> {
        if GetWindow(self.hwnd, GW_HWNDNEXT).ok() == Some(owner) {
            return Ok(());
        }
        let above = GetWindow(owner, GW_HWNDPREV).unwrap_or(HWND_TOP);
        if above != self.hwnd {
            SetWindowPos(
                self.hwnd,
                Some(above),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )?;
        }
        Ok(())
    }
    pub unsafe fn sync_affinity(&self, owner: HWND) -> Result<()> {
        let (mut source, mut current) = (0, 0);
        GetWindowDisplayAffinity(owner, &mut source)?;
        GetWindowDisplayAffinity(self.hwnd, &mut current)?;
        if source != current {
            SetWindowDisplayAffinity(self.hwnd, WINDOW_DISPLAY_AFFINITY(source))?;
        }
        Ok(())
    }
    pub unsafe fn new(owner: HWND, state: &State) -> Result<Self> {
        let route = Box::new(Route {
            owner,
            marker: state as *const State as isize,
        });
        if let Some(hwnd) = crate::desktop::application::chrome::lease_frame(&state.chrome, owner) {
            let result = Self {
                hwnd,
                _route: route,
                binding: Some(state.chrome.clone()),
            };
            windows::Win32::UI::Shell::SetWindowSubclass(
                hwnd,
                Some(borrowed_procedure),
                558,
                owner.0 as usize,
            )
            .ok()?;
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_TRANSPARENT.0 as isize));
            if SetWindowRgn(hwnd, None, false) == 0 {
                return Err(windows::core::Error::from_win32());
            }
            return Ok(result);
        }
        static CLASS_RESULT: OnceLock<std::result::Result<(), windows::core::HRESULT>> =
            OnceLock::new();
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        CLASS_RESULT
            .get_or_init(|| {
                if RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: instance,
                    lpszClassName: CLASS,
                    ..Default::default()
                }) == 0
                {
                    Err(windows::core::Error::from_win32().code())
                } else {
                    Ok(())
                }
            })
            .map_err(windows::core::Error::from)?;
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("Otter · 缩放预览"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            Some(owner),
            None,
            Some(instance),
            Some((&*route as *const Route).cast()),
        )?;
        let result = Self {
            hwnd,
            _route: route,
            binding: None,
        };
        let mut affinity = 0;
        GetWindowDisplayAffinity(owner, &mut affinity)?;
        SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(affinity))?;
        result.sync_order(owner)?;
        Ok(result)
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if let Some(binding) = &self.binding {
                if IsWindow(Some(self.hwnd)).as_bool() {
                    let _ = windows::Win32::UI::Shell::RemoveWindowSubclass(
                        self.hwnd,
                        Some(borrowed_procedure),
                        558,
                    );
                    let style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
                    SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize);
                }
                crate::desktop::application::chrome::release_frame(binding, self.hwnd);
                return;
            }
            if IsWindow(Some(self.hwnd)).as_bool() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe extern "system" fn borrowed_procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if matches!(
        msg,
        WM_NCHITTEST
            | WM_MOUSEACTIVATE
            | WM_ERASEBKGND
            | WM_MOUSEWHEEL
            | WM_MOUSEHWHEEL
            | WM_KEYDOWN
            | WM_KEYUP
            | WM_SYSKEYDOWN
            | WM_SYSKEYUP
            | WM_LBUTTONDOWN
            | WM_RBUTTONDOWN
            | WM_MBUTTONDOWN
            | WM_CONTEXTMENU
            | WM_CLOSE
    ) {
        let owner = HWND(data as *mut _);
        let marker = GetWindowLongPtrW(owner, GWLP_USERDATA);
        if marker == 0 {
            return windows::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wp, lp);
        }
        let route = Route { owner, marker };
        return std::panic::catch_unwind(|| route_dispatch(hwnd, msg, wp, lp, route))
            .unwrap_or(LRESULT(0));
    }
    windows::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wp, lp)
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    std::panic::catch_unwind(|| dispatch(hwnd, msg, wp, lp)).unwrap_or_else(|_| {
        let _ = ShowWindow(hwnd, SW_HIDE);
        LRESULT(0)
    })
}
unsafe fn dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Route;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let route = *pointer;
    route_dispatch(hwnd, msg, wp, lp, route)
}
unsafe fn route_dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, route: Route) -> LRESULT {
    if !IsWindow(Some(route.owner)).as_bool()
        || GetWindowLongPtrW(route.owner, GWLP_USERDATA) != route.marker
    {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let state = &*(route.marker as *const State);
    match msg {
        WM_NCHITTEST => LRESULT(HTCLIENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP => {
            SendMessageW(route.owner, msg, Some(wp), Some(lp))
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            let mut screen = POINT {
                x: lp.0 as i16 as i32,
                y: (lp.0 >> 16) as i16 as i32,
            };
            if !ClientToScreen(hwnd, &mut screen).as_bool() {
                return LRESULT(0);
            }
            if !super::super::stationary::active(state)
                || !super::super::stationary::eligible(state)
                || msg != WM_LBUTTONDOWN
            {
                super::super::animation::stop(route.owner, state);
            }
            if !state.lifetime.owns(route.owner) {
                return LRESULT(0);
            }
            let screen_lp = LPARAM((screen.x as u16 as isize) | ((screen.y as u16 as isize) << 16));
            let hit = SendMessageW(route.owner, WM_NCHITTEST, None, Some(screen_lp));
            let _ = SetForegroundWindow(route.owner);
            let at = std::time::Instant::now();
            let nonclient = hit.0 == HTCAPTION as isize && msg == WM_LBUTTONDOWN;
            let result = if nonclient {
                SendMessageW(
                    route.owner,
                    WM_NCLBUTTONDOWN,
                    Some(WPARAM(HTCAPTION as usize)),
                    Some(screen_lp),
                )
            } else {
                let mut point = screen;
                if !ScreenToClient(route.owner, &mut point).as_bool() {
                    return LRESULT(0);
                }
                SendMessageW(
                    route.owner,
                    msg,
                    Some(wp),
                    Some(LPARAM(
                        (point.x as u16 as isize) | ((point.y as u16 as isize) << 16),
                    )),
                )
            };
            if state.lifetime.owns(route.owner) {
                state.pin_animation.click.set(Some(Click {
                    msg: if nonclient { WM_NCLBUTTONDOWN } else { msg },
                    point: screen,
                    at,
                }));
            }
            result
        }
        WM_CONTEXTMENU | WM_CLOSE => {
            super::super::animation::stop(route.owner, state);
            if state.lifetime.owns(route.owner) {
                SendMessageW(route.owner, msg, Some(wp), Some(lp))
            } else {
                LRESULT(0)
            }
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
