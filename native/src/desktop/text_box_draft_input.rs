//! Hit and navigate the displayed DirectWrite lines. EDIT still handles text
//! mutation, clipboard, undo, word shortcuts and IME composition.
use super::*;
use windows::Win32::UI::Input::Ime::*;

pub(super) unsafe fn geometry(v: &View, m: &Model) -> Result<([f32; 2], [f32; 3])> {
    v.settle(m);
    let [mut x, mut y, mut h] = m.layout.insertion_geometry(v.active.get())?;
    if let Some(p) = v.preferred.get() {
        let [cx, cy] = m.layout.point_caret(p)?;
        let rect = m.layout.point_cluster_rect(p)?;
        x = cx;
        h = rect.bottom - rect.top;
        y = cy - h * 0.5;
    }
    let mut rect = RECT::default();
    GetClientRect(v.edit, &mut rect)?;
    let mut offset = v.offset.get();
    for (i, (p, extent, space)) in [(x, 1. / v.scale, rect.right), (y, h, rect.bottom)]
        .into_iter()
        .enumerate()
    {
        let space = (space as f32 / v.scale - 2. * v.pad).max(1.);
        offset[i] = offset[i].min(p).max(p + extent.min(space) - space).max(0.);
    }
    v.offset.set(offset);
    Ok(([v.pad - offset[0], v.pad - offset[1]], [x, y, h]))
}

pub(super) unsafe fn before(v: &View, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    if msg == WM_CAPTURECHANGED || msg == WM_CANCELMODE {
        v.drag.set(false);
    }
    if msg == WM_LBUTTONUP && v.drag.replace(false) {
        if GetCapture() == v.edit {
            let _ = ReleaseCapture();
        }
        return Some(LRESULT(0));
    }
    let pointer =
        matches!(msg, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) || msg == WM_MOUSEMOVE && v.drag.get();
    let key = wp.0 as u16;
    let movement = msg == WM_KEYDOWN
        && GetKeyState(VK_CONTROL.0 as i32) >= 0
        && [VK_UP, VK_DOWN, VK_HOME, VK_END, VK_LEFT, VK_RIGHT]
            .iter()
            .any(|k| k.0 == key);
    if !pointer && !movement {
        return None;
    }
    let m = v.frame.borrow().clone()?;
    // Native IME consumes candidate navigation. Do not change its selection.
    if movement && v.composing.get() {
        return None;
    }
    let result = (|| -> Result<()> {
        let (lo, hi) = v.settle(&m);
        let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
        let active = v.active.get();
        let (next, affinity) = if pointer {
            let [ox, oy] = [v.pad - v.offset.get()[0], v.pad - v.offset.get()[1]];
            let pt = [
                (lp.0 as i16 as f32) / v.scale - ox,
                ((lp.0 >> 16) as i16 as f32) / v.scale - oy,
            ];
            let hit = m.layout.hit_text(pt)?;
            if msg == WM_LBUTTONDBLCLK {
                let word = m
                    .layout
                    .word_range(hit.position, &m.layout.word_boundaries()?)?;
                v.preferred.set(None);
                v.select(&m, word.start, word.end);
                return Ok(());
            }
            if msg == WM_LBUTTONDOWN {
                let _ = SetFocus(Some(v.edit));
                if !v.owns(v.edit) {
                    return Ok(());
                }
                SetCapture(v.edit);
                if !v.owns(v.edit) {
                    return Ok(());
                }
                v.drag.set(GetCapture() == v.edit);
            }
            (hit.position, Some(m.layout.point_caret(pt)?))
        } else if key == VK_UP.0 || key == VK_DOWN.0 {
            let (p, pt) = m
                .layout
                .vertical_text(active, key == VK_DOWN.0, v.preferred.get())?;
            (p, Some(pt))
        } else if key == VK_HOME.0 || key == VK_END.0 {
            let (p, pt) = m
                .layout
                .line_edge(active, key == VK_END.0, v.preferred.get())?;
            (p, Some(pt))
        } else {
            let forward = key == VK_RIGHT.0;
            let next = if lo != hi && !shift {
                if forward {
                    hi
                } else {
                    lo
                }
            } else {
                m.layout.step_text(active, forward)?
            };
            (next, None)
        };
        let anchor = if shift || msg == WM_MOUSEMOVE {
            v.anchor.get()
        } else {
            next
        };
        v.preferred.set(affinity);
        v.select(&m, anchor, next);
        Ok(())
    })();
    if let Err(err) = result {
        eprintln!("native-boxed-text-input: {err}");
    }
    Some(LRESULT(0))
}

pub(super) unsafe fn caret(v: &View) {
    if GetFocus() != v.edit || v.positioning.replace(true) {
        return;
    }
    struct Busy<'a>(&'a Cell<bool>);
    impl Drop for Busy<'_> {
        fn drop(&mut self) {
            self.0.set(false);
        }
    }
    let _busy = Busy(&v.positioning);
    let Some(m) = v.frame.borrow().clone() else {
        return;
    };
    let Ok(([ox, oy], [x, y, h])) = geometry(v, &m) else {
        return;
    };
    let pt = POINT {
        x: ((ox + x) * v.scale).round() as i32,
        y: ((oy + y) * v.scale).round() as i32,
    };
    let rect = RECT {
        left: pt.x,
        top: pt.y,
        right: pt.x.saturating_add(1),
        bottom: pt.y.saturating_add((h * v.scale).ceil() as i32),
    };
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // Only reposition this EDIT's caret, never the thread's next focus owner.
    if GetGUIThreadInfo(
        windows::Win32::System::Threading::GetCurrentThreadId(),
        &mut info,
    )
    .is_ok()
        && info.hwndCaret == v.edit
    {
        let _ = SetCaretPos(pt.x, pt.y);
    }
    if !v.composing.get() || !v.owns(v.edit) || GetFocus() != v.edit {
        return;
    }
    let context = ImmGetContext(v.edit);
    if context.0.is_null() {
        return;
    }
    let _ = ImmSetCompositionWindow(
        context,
        &COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: pt,
            rcArea: rect,
        },
    );
    let _ = ImmSetCandidateWindow(
        context,
        &CANDIDATEFORM {
            dwIndex: 0,
            dwStyle: CFS_EXCLUDE,
            ptCurrentPos: POINT {
                x: pt.x,
                y: rect.bottom,
            },
            rcArea: rect,
        },
    );
    let _ = ImmReleaseContext(v.edit, context);
}
