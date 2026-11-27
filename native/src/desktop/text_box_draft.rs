//! The EDIT owns the text store, IME and undo. Only boxed annotations cover
//! its GDI client with a DirectWrite view; both HWNDs share one session owner.
use super::*;
use std::cell::{Cell, RefCell};
use windows::Win32::UI::{Controls::*, Shell::*};
#[path = "text_box_draft_paint.rs"]
mod draw;
#[path = "text_box_draft_input.rs"]
mod input;

const ID: usize = 0x4f544257;
pub(super) struct Model {
    layout: crate::surface_text::Layout,
    // Only original CRLF pairs add one native selection offset. Lone LF/CR
    // and soft wraps never insert extra units into the native text store.
    lf: Vec<(u32, u32)>,
}
impl Model {
    fn native(&self, p: u32) -> u32 {
        p + self.lf.partition_point(|&(model, _)| model < p) as u32
    }
    fn model(&self, p: u32) -> u32 {
        p.saturating_sub(self.lf.partition_point(|&(_, cr)| cr < p) as u32)
            .min(self.layout.text_len())
    }
}
pub(super) struct View {
    edit: HWND,
    cover: HWND,
    factory: windows::Win32::Graphics::Direct2D::ID2D1Factory,
    scale: f32,
    pad: f32,
    keyed: bool,
    frame: RefCell<Option<Rc<Model>>>,
    anchor: Cell<u32>,
    active: Cell<u32>,
    drag: Cell<bool>,
    preferred: Cell<Option<[f32; 2]>>,
    offset: Cell<[f32; 2]>,
    positioning: Cell<bool>,
    composing: Cell<bool>,
}
impl View {
    pub(super) unsafe fn install(
        edit: HWND,
        scale: f32,
        pad: f32,
        keyed: bool,
    ) -> Result<Rc<Self>> {
        use windows::Win32::Graphics::Direct2D::*;
        let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let cover = CreateWindowExW(
            WS_EX_NOACTIVATE,
            w!("STATIC"),
            w!(""),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            1,
            1,
            Some(edit),
            None,
            None,
            None,
        )?;
        let view = Rc::new(Self {
            edit,
            cover,
            factory,
            scale,
            pad,
            keyed,
            frame: Default::default(),
            anchor: Cell::new(0),
            active: Cell::new(0),
            drag: Cell::new(false),
            preferred: Cell::new(None),
            offset: Cell::new([0.; 2]),
            positioning: Cell::new(false),
            composing: Cell::new(false),
        });
        for hwnd in [edit, cover] {
            let raw = Rc::into_raw(view.clone());
            if !SetWindowSubclass(hwnd, Some(route), ID, raw as usize).as_bool() {
                drop(Rc::from_raw(raw));
                return Err(Error::from_win32());
            }
        }
        Ok(view)
    }
    pub(super) unsafe fn update(
        &self,
        text: &TextAnnotation,
        lf: Vec<(u32, u32)>,
    ) -> Result<Point> {
        // Drop the old visual on a failed reshape rather than publishing stale
        // characters as the current input. The editable text remains intact.
        self.clear();
        let units: Vec<_> = text.content().encode_utf16().collect();
        let native =
            desktop::annotation_text::layout_units(text, 1., 1_000_000., 1_000_000., &units)?;
        let size = desktop::annotation_text::measured(text, &native)?;
        let layout = crate::surface_text::Layout::from_annotation(native, units)?;
        *self.frame.borrow_mut() = Some(Rc::new(Model { layout, lf }));
        self.preferred.set(None);
        Ok(size)
    }
    pub(super) fn clear(&self) {
        self.frame.borrow_mut().take();
        self.invalidate();
    }
    pub(super) fn invalidate(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.cover), None, false);
        }
    }
    unsafe fn owns(&self, hwnd: HWND) -> bool {
        let mut data = 0;
        GetWindowSubclass(hwnd, Some(route), ID, Some(&mut data)).as_bool()
            && data == self as *const _ as usize
    }
    pub(super) unsafe fn place(&self, width: i32, height: i32) -> Result<()> {
        SetWindowPos(
            self.cover,
            Some(HWND_TOP),
            0,
            0,
            width,
            height,
            SWP_NOACTIVATE,
        )?;
        self.invalidate();
        Ok(())
    }
    unsafe fn selected(&self, model: &Model) -> (u32, u32) {
        let (mut a, mut b) = (0u32, 0u32);
        SendMessageW(
            self.edit,
            EM_GETSEL,
            Some(WPARAM(&mut a as *mut _ as usize)),
            Some(LPARAM(&mut b as *mut _ as isize)),
        );
        (model.model(a), model.model(b))
    }
    unsafe fn settle(&self, model: &Model) -> (u32, u32) {
        let (a, b) = self.selected(model);
        let active = self.active.get();
        if (a, b) != (self.anchor.get().min(active), self.anchor.get().max(active)) {
            self.anchor.set(a);
            self.active.set(b);
            self.preferred.set(None);
        }
        (a, b)
    }
    unsafe fn select(&self, m: &Model, anchor: u32, active: u32) {
        self.anchor.set(anchor);
        self.active.set(active);
        SendMessageW(
            self.edit,
            EM_SETSEL,
            Some(WPARAM(m.native(anchor) as usize)),
            Some(LPARAM(m.native(active) as isize)),
        );
        self.invalidate();
    }
}

unsafe extern "system" fn route(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let raw = data as *const View;
    Rc::increment_strong_count(raw);
    let view = Rc::from_raw(raw);
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(route), ID);
        drop(Rc::from_raw(raw));
        return DefSubclassProc(hwnd, msg, wp, lp);
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if hwnd == view.cover {
            return match msg {
                WM_PAINT | WM_PRINTCLIENT => draw::paint(&view, msg, wp),
                WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
                WM_ERASEBKGND => LRESULT(1),
                _ => DefSubclassProc(hwnd, msg, wp, lp),
            };
        }
        if matches!(msg, WM_IME_STARTCOMPOSITION | WM_IME_ENDCOMPOSITION) {
            view.composing.set(msg == WM_IME_STARTCOMPOSITION);
        }
        if desktop::session_end::blocks_input(msg) {
            return LRESULT(0);
        }
        if let Some(result) = input::before(&view, msg, wp, lp) {
            return result;
        }
        let result = DefSubclassProc(hwnd, msg, wp, lp);
        if view.owns(hwnd) && view.owns(view.cover) {
            if msg == WM_SETTEXT {
                // Multiline EDIT omits EN_CHANGE for WM_SETTEXT.
                if let Ok(parent) = GetParent(hwnd) {
                    messages::send(parent, messages::CHANGE, false);
                }
            }
            if matches!(
                msg,
                WM_SETFOCUS
                    | WM_KILLFOCUS
                    | WM_CHAR
                    | WM_PASTE
                    | WM_CUT
                    | WM_CLEAR
                    | WM_UNDO
                    | WM_KEYDOWN
                    | EM_SETSEL
                    | WM_IME_STARTCOMPOSITION
                    | WM_IME_COMPOSITION
                    | WM_IME_ENDCOMPOSITION
            ) {
                view.invalidate();
                input::caret(&view);
            }
        }
        result
    }))
    .unwrap_or_else(|_| {
        PostQuitMessage(1);
        LRESULT(0)
    })
}
