//! System pan/inertia-message seam; hold one local sequence without custom inertia threads/timers.
use super::*;
use windows::Win32::System::SystemServices::*;
use windows::Win32::UI::Input::Touch::*;
#[derive(Clone)]
pub(super) struct Pan {
    target: HWND,
    anchor: String,
    sequence: u32,
    last: [i32; 2],
}
pub(super) fn clear(state: &State) {
    state.pan.borrow_mut().take();
}
pub(super) unsafe fn configure(hwnd: HWND) -> Result<()> {
    SetGestureConfig(
        hwnd,
        0,
        &[GESTURECONFIG {
            dwID: GID_PAN,
            dwWant: (GC_PAN
                | GC_PAN_WITH_SINGLE_FINGER_VERTICALLY
                | GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY
                | GC_PAN_WITH_INERTIA)
                .0,
            dwBlock: GC_PAN_WITH_GUTTER.0,
        }],
        std::mem::size_of::<GESTURECONFIG>() as u32,
    )
}
struct OwnedGesture(HGESTUREINFO);
/// System default handling forwards child-HWND gestures but keeps the child as target; admit only currently mounted Sliders.
pub(super) fn accepts_target(hwnd: HWND, state: &State, target: HWND) -> bool {
    if target == hwnd {
        return true;
    }
    if unsafe { GetParent(target) }.ok() != Some(hwnd) {
        return false;
    }
    state.host.try_borrow().ok().is_some_and(|host| {
        host.as_ref().is_some_and(|host| {
            host.accessibility_edit_id(target)
                .is_some_and(|id| host.slider_value(&id).is_some())
        })
    })
}
impl Drop for OwnedGesture {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseGestureInfoHandle(self.0);
        }
    }
}
pub(super) unsafe fn message(hwnd: HWND, state: &State, lp: LPARAM) -> bool {
    let handle = HGESTUREINFO(lp.0 as *mut _);
    let mut info = GESTUREINFO {
        cbSize: std::mem::size_of::<GESTUREINFO>() as u32,
        ..Default::default()
    };
    if GetGestureInfo(handle, &mut info).is_err() {
        return false;
    }
    // BEGIN/END also require identity checks first; other children cannot cancel current candidates or scroll sequences.
    if !accepts_target(hwnd, state, info.hwndTarget) {
        // Revoke this sequence if the mount disappears or Host is temporarily busy with reentrancy; do not accumulate missed movement for later messages.
        if state
            .pan
            .borrow()
            .as_ref()
            .is_some_and(|pan| pan.target == info.hwndTarget && pan.sequence == info.dwSequenceID)
        {
            clear(state);
        }
        return false;
    }
    // Generic BEGIN/END and other gestures still reach DefWindowProc, whose default handler closes their handles.
    if info.dwID == GID_BEGIN.0 {
        pointer::invalidate(hwnd, state);
        return false;
    }
    if info.dwID == GID_END.0 {
        if state
            .pan
            .borrow()
            .as_ref()
            .is_some_and(|p| p.sequence == info.dwSequenceID && p.target == info.hwndTarget)
        {
            clear(state);
        }
        return false;
    }
    if info.dwID != GID_PAN.0 {
        return false;
    }
    let _handle = OwnedGesture(handle);
    let mut point = POINT {
        x: info.ptsLocation.x.into(),
        y: info.ptsLocation.y.into(),
    };
    if !ScreenToClient(hwnd, &mut point).as_bool() {
        clear(state);
        return true;
    }
    pan_from(
        hwnd,
        state,
        info.hwndTarget,
        info.dwSequenceID,
        info.dwFlags,
        [point.x, point.y],
    );
    true
}
/// A controllable input seam verifies the state machine; production coordinates come only from successfully decoded system GESTUREINFO.
#[cfg(test)]
pub(super) fn pan(hwnd: HWND, state: &State, sequence: u32, flags: u32, point: [i32; 2]) {
    pan_from(hwnd, state, hwnd, sequence, flags, point);
}
pub(super) fn pan_from(
    hwnd: HWND,
    state: &State,
    target: HWND,
    sequence: u32,
    flags: u32,
    point: [i32; 2],
) {
    if state.closed.get() || !state.local_edit.get() {
        clear(state);
        return;
    }
    if flags & GF_BEGIN != 0 {
        pointer::invalidate(hwnd, state);
        let mut anchor = None;
        let result = state.run(|host| {
            anchor = host.pan_anchor(point)?;
            Ok(())
        });
        match result {
            Ok(true) if !state.closed.get() && state.local_edit.get() && flags & GF_END == 0 => {
                *state.pan.borrow_mut() = anchor.map(|anchor| Pan {
                    target,
                    anchor,
                    sequence,
                    last: point,
                });
            }
            Err(error) => *state.error.borrow_mut() = Some(error),
            _ => {}
        }
        return;
    }
    let Some(mut pan) = state
        .pan
        .borrow()
        .clone()
        .filter(|p| p.sequence == sequence && p.target == target)
    else {
        return;
    };
    let Some(delta) = pan.last[0]
        .checked_sub(point[0])
        .zip(pan.last[1].checked_sub(point[1]))
    else {
        clear(state);
        return;
    };
    pan.last = point;
    // Update coordinates first; do not accumulate displacement while Host is busy or restore old sequences after reentrant cancellation.
    *state.pan.borrow_mut() = if flags & GF_END == 0 {
        Some(pan.clone())
    } else {
        None
    };
    let mut changed = false;
    let result = state.run(|host| {
        changed = host.pan(&pan.anchor, [delta.0, delta.1])?;
        Ok(())
    });
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        pointer::invalidate(hwnd, state);
        changed = true;
    }
    if changed {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}
