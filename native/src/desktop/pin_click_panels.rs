//! Visibility guard for owned top-level UI surfaces (including re-show requests
//! from capture cleanup). The subclass keeps no borrowed Rust pointer.
use super::*;
use std::{collections::HashMap, rc::Weak};
use windows::Win32::UI::{Input::KeyboardAndMouse::GetFocus, Shell::*};
const SUBCLASS: usize = 0x21e3;

#[derive(Clone)]
struct Binding {
    panel: HWND,
    host: HWND,
    identity: Weak<()>,
    strip: bool,
}
thread_local! {
    static KEYS: Cell<usize> = const { Cell::new(0) };
    static BINDINGS: RefCell<HashMap<usize, Binding>> = RefCell::default();
}
unsafe fn key(panel: HWND) -> Option<usize> {
    let mut k = 0;
    GetWindowSubclass(panel, Some(filter), SUBCLASS, Some(&mut k))
        .as_bool()
        .then_some(k)
}
pub(in crate::desktop) unsafe fn bind(
    panel: HWND,
    host: HWND,
    s: &State,
    strip: bool,
) -> Result<()> {
    if !owns(host, s) || GetWindowThreadProcessId(panel, None) != GetCurrentThreadId() {
        return Err(unavailable());
    }
    if key(panel).is_some() {
        return binding(panel, s)
            .filter(|(_, b)| b.strip == strip)
            .map(|_| ())
            .ok_or_else(unavailable);
    }
    let k = KEYS.with(|seq| {
        let k = seq.get().checked_add(1).ok_or_else(unavailable)?;
        seq.set(k);
        Ok::<_, Error>(k)
    })?;
    SetWindowSubclass(panel, Some(filter), SUBCLASS, k).ok()?;
    BINDINGS.with_borrow_mut(|m| {
        m.insert(
            k,
            Binding {
                panel,
                host,
                identity: Rc::downgrade(&s.click.identity),
                strip,
            },
        )
    });
    Ok(())
}
unsafe fn binding(panel: HWND, s: &State) -> Option<(usize, Binding)> {
    let k = key(panel)?;
    let b = BINDINGS.with_borrow(|m| m.get(&k).cloned())?;
    (b.panel == panel && b.identity.ptr_eq(&Rc::downgrade(&s.click.identity)) && owns(b.host, s))
        .then_some((k, b))
}
pub(super) unsafe fn toolbar(s: &State) -> Option<(HWND, usize)> {
    let h = s.tool_panel.get();
    binding(h, s).map(|(key, _)| (h, key))
}
pub(super) unsafe fn property(s: &State) -> Option<HWND> {
    let panel = s.tool_panel.get();
    let (_, b) = binding(panel, s)?;
    b.strip
        .then(|| neuro_mini_ui::property_window(panel))
        .flatten()
}
unsafe extern "system" fn filter(
    panel: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    k: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(panel, Some(filter), id);
        BINDINGS.with_borrow_mut(|m| m.remove(&k));
    }
    if msg == WM_WINDOWPOSCHANGING && lp.0 != 0 {
        let p = &mut *(lp.0 as *mut WINDOWPOS);
        if let Some(b) = BINDINGS
            .with_borrow(|m| m.get(&k).cloned())
            .filter(|b| p.flags.contains(SWP_SHOWWINDOW) && b.panel == panel)
        {
            let hwnd = b.host;
            if GetClassLongPtrW(hwnd, GCLP_WNDPROC)
                == crate::desktop::procedure::window_proc as *const () as usize
                && GetWindowThreadProcessId(hwnd, None) == GetCurrentThreadId()
            {
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
                if state.as_ref().is_some_and(|s| {
                    b.identity.ptr_eq(&Rc::downgrade(&s.click.identity))
                        && s.lifetime.owns(hwnd)
                        && s.click.active()
                }) {
                    p.flags = (p.flags & !SWP_SHOWWINDOW) | SWP_HIDEWINDOW;
                }
            }
        }
    }
    DefSubclassProc(panel, msg, wp, lp)
}
pub(in crate::desktop) unsafe fn idle(s: &State) -> bool {
    if !super::super::pin_settings::connection_popup_idle(s) {
        return false;
    }
    let focus = GetFocus();
    let h = s.tool_panel.get();
    if !h.is_invalid() && toolbar(s).is_none() {
        return false;
    }
    [
        toolbar(s).map(|(h, _)| h),
        property(s),
        super::super::pin_settings::window(s),
    ]
    .into_iter()
    .flatten()
    .all(|h| focus != h && !IsChild(h, focus).as_bool())
}
