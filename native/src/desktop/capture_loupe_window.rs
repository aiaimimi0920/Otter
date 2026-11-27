//! One input-transparent popup per picker. Sampling uses physical coordinates and a bounded patch.
use super::{capture, capture_pixel, image::Image, Window};
use crate::{
    capture_loupe_layout::{place, Bounds},
    color_format::Format,
};
use std::{
    cell::{Cell, OnceCell},
    rc::Rc,
};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::GetDpiForWindow, Shell::*, WindowsAndMessaging::*},
    },
};
#[path = "capture_loupe_paint.rs"]
mod paint;
pub(super) const UPDATE: u32 = WM_APP + 0x663;
pub(super) type Grid = [Option<[u8; 3]>; 225];
pub(super) struct Hud {
    picture: Rc<paint::Picture>,
    displays: Vec<Bounds>,
    hidden: Cell<bool>,
    queued: Cell<bool>,
}
impl Hud {
    pub(super) unsafe fn new(owner: HWND) -> Result<Self> {
        let displays = super::capture_monitors::snapshot()?;
        let window = Window(CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            w!("STATIC"),
            w!(""),
            WS_POPUP | WS_DISABLED,
            0,
            0,
            1,
            1,
            Some(owner),
            None,
            None,
            None,
        )?);
        SetPropW(window.0, w!("NonRudeHWND"), Some(HANDLE(1usize as *mut _)))?;
        SetWindowDisplayAffinity(window.0, WDA_EXCLUDEFROMCAPTURE)?;
        SetLayeredWindowAttributes(window.0, COLORREF(0), 255, LWA_ALPHA)?;
        let picture = Rc::new(paint::Picture::default());
        paint::attach(window.0, picture.clone())?;
        picture.window.set(window.0);
        // WM_NCDESTROY clears the shared identity even if Windows destroys an
        // owned popup first. No later drop may destroy a recycled HWND.
        std::mem::forget(window);
        Ok(Self {
            picture,
            displays,
            hidden: Cell::new(false),
            queued: Cell::new(false),
        })
    }
    pub(super) fn hwnd(&self) -> HWND {
        self.picture.window.get()
    }
    pub(super) unsafe fn hide(&self) {
        let _ = ShowWindow(self.hwnd(), SW_HIDE);
    }
    pub(super) unsafe fn toggle(&self) {
        self.hidden.set(!self.hidden.get());
        if self.hidden.get() {
            self.hide();
        }
    }
    pub(super) fn format(&self) -> Format {
        self.picture.format.get()
    }
    pub(super) fn cycle(&self) {
        self.picture.format.set(self.format().next());
    }
    pub(super) unsafe fn schedule(&self, owner: HWND) {
        if !self.hidden.get()
            && !self.queued.replace(true)
            && PostMessageW(Some(owner), UPDATE, WPARAM(0), LPARAM(0)).is_err()
        {
            self.queued.set(false);
        }
    }
    fn monitor(&self, x: i32, y: i32) -> Option<Bounds> {
        self.displays
            .iter()
            .copied()
            .find(|[l, t, r, b]| x >= *l && x < *r && y >= *t && y < *b)
    }
    pub(super) fn valid(&self, p: POINT) -> bool {
        self.monitor(p.x, p.y).is_some()
    }
    pub(super) fn displays(&self) -> &[Bounds] {
        &self.displays
    }
    pub(super) unsafe fn refresh(
        &self,
        pointer: POINT,
        selection: Option<Bounds>,
        frozen: Option<(&Image, POINT)>,
    ) -> Result<()> {
        self.queued.set(false);
        if self.hidden.get() {
            return Ok(());
        }
        let Some(monitor) = self.monitor(pointer.x, pointer.y) else {
            self.hide();
            return Ok(());
        };
        let start = POINT {
            x: pointer.x.saturating_sub(7),
            y: pointer.y.saturating_sub(7),
        };
        // Capture at most 900 pixel bytes. All three visible capture layers are
        // excluded; the final C/commit independently confirms the current center.
        let live;
        let (image, origin) = if let Some(source) = frozen {
            source
        } else {
            live = capture::region(start.x, start.y, 15, 15)?;
            (&live, start)
        };
        let grid = std::array::from_fn(|i| {
            let x = start.x.saturating_add((i % 15) as i32);
            let y = start.y.saturating_add((i / 15) as i32);
            self.monitor(x, y)?;
            capture_pixel::valid_rgb(
                image,
                POINT {
                    x: x.checked_sub(origin.x)?,
                    y: y.checked_sub(origin.y)?,
                },
            )
        });
        self.picture.grid.set(grid);
        let hwnd = self.hwnd();
        let mut dpi = GetDpiForWindow(hwnd);
        for _ in 0..2 {
            let [l, t, r, b] = place(monitor, [pointer.x, pointer.y], selection, dpi);
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                l,
                t,
                r - l,
                b - t,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )?;
            let next = GetDpiForWindow(hwnd);
            if next == dpi {
                break;
            }
            dpi = next;
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
        Ok(())
    }
}
impl Drop for Hud {
    fn drop(&mut self) {
        let hwnd = self.hwnd();
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
// The two selectors share all HUD handling; only their commit semantics differ.
#[derive(Default)]
pub(super) struct Selector(OnceCell<Hud>);
impl Selector {
    pub(super) unsafe fn install(&self, hwnd: HWND) -> Result<()> {
        self.0
            .set(Hud::new(hwnd)?)
            .map_err(|_| Error::new(E_UNEXPECTED, "Duplicate picker HUD"))
    }
    pub(super) fn get(&self) -> Option<&Hud> {
        self.0.get()
    }
    pub(super) unsafe fn prime(&self, hwnd: HWND, pointer: &Cell<Option<POINT>>, origin: POINT) {
        let mut p = POINT::default();
        if GetCursorPos(&mut p).is_ok() {
            pointer.set(Some(POINT {
                x: p.x.saturating_sub(origin.x),
                y: p.y.saturating_sub(origin.y),
            }));
            self.schedule(hwnd);
        }
    }
    pub(super) unsafe fn schedule(&self, hwnd: HWND) {
        if let Some(h) = self.get() {
            h.schedule(hwnd)
        }
    }
    pub(super) unsafe fn dispatch(
        &self,
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
        point: Option<POINT>,
        selection: Option<RECT>,
        origin: POINT,
        image: Option<&Image>,
    ) -> bool {
        let Some(h) = self.get() else { return false };
        match msg {
            WM_KEYDOWN if matches!(wp.0, 0x5a | 0x09) => {
                if lp.0 & (1 << 30) == 0 {
                    if wp.0 == 0x5a {
                        h.toggle();
                    } else {
                        h.cycle();
                    }
                    h.schedule(hwnd);
                }
                true
            }
            UPDATE => {
                if let Some(p) = point {
                    let rect = selection.map(|r| {
                        [
                            r.left.saturating_add(origin.x),
                            r.top.saturating_add(origin.y),
                            r.right.saturating_add(origin.x),
                            r.bottom.saturating_add(origin.y),
                        ]
                    });
                    if let Err(e) = h.refresh(
                        POINT {
                            x: p.x.saturating_add(origin.x),
                            y: p.y.saturating_add(origin.y),
                        },
                        rect,
                        image.map(|i| (i, origin)),
                    ) {
                        h.hide();
                        eprintln!("native-picker-hud: {e}");
                    }
                } else {
                    h.queued.set(false);
                }
                true
            }
            _ => false,
        }
    }
}
