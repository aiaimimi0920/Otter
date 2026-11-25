//! Source-thread menu placement/z-order changes and return; same-HWND reentry must preserve recovery identity without regranting display leases.
use crate::{native_popup::Lease, native_protocol::Request};
use std::cell::RefCell;
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub const TEMPORARY: PCWSTR = w!("Otter.Live.Popup.TemporaryTopmost.v1");
pub const MANAGED: PCWSTR = w!("Otter.Live.Popup.ManagedTopmost.v1");
const MAX_PROMOTED: usize = 4;

thread_local! {
    static PROMOTED: RefCell<Vec<Lease>> = const { RefCell::new(Vec::new()) };
    static MUTATING: RefCell<Vec<Lease>> = const { RefCell::new(Vec::new()) };
}

struct Mutation(Lease);
impl Mutation {
    fn for_request(lease: Lease) -> windows::core::Result<Self> {
        // Both rejection cases, same-HWND reentry and full bounded slots, precede source-window mutation.
        // Fixed requests use explicit Pending to distinguish settled busy state from unknown in-flight results.
        Self::enter(lease).map_err(|_| {
            windows::core::Error::new(
                crate::native_layer_protocol::PENDING,
                "Popup window mutation is pending",
            )
        })
    }
    fn enter(lease: Lease) -> windows::core::Result<Self> {
        MUTATING.with(|entries| {
            let mut entries = entries.borrow_mut();
            if entries.len() >= 16 || entries.iter().any(|entry| entry.popup == lease.popup) {
                return Err(windows::core::Error::new(
                    E_ABORT,
                    "Popup window mutation reentered",
                ));
            }
            entries.push(lease);
            Ok(Self(lease))
        })
    }
}
impl Drop for Mutation {
    fn drop(&mut self) {
        MUTATING.with(|entries| entries.borrow_mut().retain(|entry| *entry != self.0));
    }
}

pub(crate) fn mutating(popup: HWND) -> bool {
    MUTATING.with(|entries| entries.borrow().iter().any(|entry| entry.popup == popup))
}

fn set_topmost(popup: HWND, enabled: bool) -> windows::core::Result<()> {
    unsafe {
        SetWindowPos(
            popup,
            Some(if enabled {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            }),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )?;
        // Desktop policy/source window procedures may ignore Z-order; verify actual results even after Win32 success.
        if (GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0) != enabled {
            return Err(windows::core::Error::new(
                E_FAIL,
                "Popup topmost change was not applied",
            ));
        }
    }
    Ok(())
}

pub(crate) fn prepare(lease: Lease) -> windows::core::Result<()> {
    if !lease.valid() {
        return Ok(());
    }
    let result = {
        // Placement also synchronously enters applications; protect this display generation before placement, rather than
        // rereading a potentially hide/show-replaced lease after placement returns.
        let _mutation = Mutation::enter(lease)?;
        (|| {
            crate::native_dispatch::place_popup(lease.popup)?;
            promote(lease)
        })()
    };
    if result.is_err() || !lease.valid() {
        // Reentrant hide revokes input without clearing identity. After outer mutation returns, restore the actually borrowed
        // z-order; originally topmost/placement-only windows have no protocol marker and must not be demoted.
        let tracked = PROMOTED.with(|entries| {
            let mut entries = entries.borrow_mut();
            let before = entries.len();
            entries.retain(|entry| *entry != lease);
            before != entries.len()
        });
        if tracked
            || [TEMPORARY, MANAGED]
                .iter()
                .any(|key| crate::native_protocol::value(lease.popup, *key) == lease.instance)
        {
            restore(lease)?;
        }
    }
    result
}

fn promote(lease: Lease) -> windows::core::Result<()> {
    unsafe {
        if !lease.valid()
            || GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE.0 == 0
            || GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0
            || GetWindowLongPtrW(lease.root, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 == 0
        {
            return Ok(());
        }
        let retained = PROMOTED.with(|entries| {
            let mut entries = entries.borrow_mut();
            if entries.len() >= MAX_PROMOTED {
                return false;
            }
            entries.push(lease);
            true
        });
        if !retained {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x800700AAu32 as i32,
            )));
        }
        // Store state in slots first, without borrows across Win32: SetWindowPos can reenter hide/destroy.
        SetPropW(
            lease.popup,
            TEMPORARY,
            Some(HANDLE(lease.instance as *mut _)),
        )?;
        set_topmost(lease.popup, true)?;
    }
    Ok(())
}

fn restore(lease: Lease) -> windows::core::Result<()> {
    // Cancellation may revoke B epoch first, so use source display-instance identity against HWND reuse without requiring valid control.
    if !lease.same_instance() {
        return Ok(());
    }
    // SetWindowPos synchronously reenters applications. Inner hide/show must retain identity and reject reauthorization,
    // without clearing recovery markers or recursively restoring the same HWND before outer restoration is confirmed.
    let _mutation = Mutation::enter(lease)?;
    restore_inner(lease)
}

// Callers already hold original-display-instance Mutation; do not recursively enter guards or hold Vec borrows across Win32.
fn restore_inner(lease: Lease) -> windows::core::Result<()> {
    unsafe {
        if GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 {
            set_topmost(lease.popup, false).or_else(|error| {
                if lease.same_instance() {
                    Err(error)
                } else {
                    Ok(())
                }
            })?;
        }
        if lease.same_instance() {
            let _ = RemovePropW(lease.popup, TEMPORARY);
            let _ = RemovePropW(lease.popup, MANAGED);
        }
    }
    Ok(())
}

pub(crate) fn acquire_managed(
    lease: Lease,
    active: impl Fn() -> bool,
) -> windows::core::Result<()> {
    let changed = || windows::core::Error::new(E_ABORT, "Popup layer request changed");
    if !lease.valid() {
        return Err(changed());
    }
    let _mutation = Mutation::for_request(lease)?;
    if !active() {
        return Err(changed());
    }
    unsafe {
        let style = GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32;
        let topmost = style & WS_EX_TOPMOST.0 != 0;
        if style & WS_EX_NOACTIVATE.0 == 0
            || GetWindowLongPtrW(lease.root, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 == 0
            || topmost
                && ![TEMPORARY, MANAGED]
                    .iter()
                    .any(|key| crate::native_protocol::value(lease.popup, *key) == lease.instance)
        {
            return Err(changed()); // The original application's own TOPMOST cannot be registered as borrowed.
        }
        SetPropW(lease.popup, MANAGED, Some(HANDLE(lease.instance as *mut _)))?;
        let result = if topmost {
            Ok(())
        } else {
            set_topmost(lease.popup, true)
        };
        if result.is_err() || !active() || !lease.valid() {
            if lease.same_instance()
                && crate::native_protocol::value(lease.popup, MANAGED) == lease.instance
            {
                restore_inner(lease)?;
            }
            return result.and_then(|()| Err(changed()));
        }
    }
    Ok(())
}

pub(crate) fn restore_managed(lease: Lease) -> windows::core::Result<()> {
    if !lease.same_instance() {
        return Ok(()); // The original object disappeared; never change z-order of subsequently displayed instances.
    }
    let _mutation = Mutation::for_request(lease)?;
    if crate::native_protocol::value(lease.popup, MANAGED) != lease.instance {
        return Ok(()); // Before takeover, never reclaim temporary z-order from in-flight source transactions.
    }
    restore_inner(lease)
}

pub(crate) fn release(popup: HWND, destroying: bool) -> bool {
    if !destroying && mutating(popup) {
        return false;
    }
    let lease = PROMOTED.with(|entries| {
        let mut entries = entries.borrow_mut();
        let index = entries.iter().position(|lease| lease.popup == popup)?;
        Some(entries.swap_remove(index))
    });
    let lease = lease.or_else(|| {
        let lease = Lease::snapshot(popup);
        (lease.same_instance()
            && [MANAGED, TEMPORARY]
                .iter()
                .any(|key| crate::native_protocol::value(popup, *key) == lease.instance))
        .then_some(lease)
    });
    if let Some(lease) = lease {
        if destroying {
            unsafe {
                let _ = RemovePropW(popup, MANAGED);
                let _ = RemovePropW(popup, TEMPORARY);
            }
        } else if let Err(error) = restore(lease) {
            eprintln!("native-live-popup-layer-restore: {error}");
            return false; // Preserve window display identity/recovery markers; revoking first must not lose responsibility.
        }
    }
    true
}

fn take_finished(entries: &mut Vec<Lease>, request: Request) -> [Option<Lease>; MAX_PROMOTED] {
    // Same limit as slot admission; retain unfinished-transaction capacity and original ordering on both sides.
    let mut finished = [None; MAX_PROMOTED];
    let mut next = 0;
    entries.retain(|lease| {
        if lease.owner != request.owner
            || lease.cookie != request.cookie
            || lease.epoch != request.epoch
            || lease.request != request.nonce
        {
            return true;
        }
        finished[next] = Some(*lease);
        next += 1;
        false
    });
    finished
}

pub(crate) fn finish(request: Request) {
    // Submenu-button return must not restore still-running parent-menu z-order; hold no borrows across window calls.
    let leases = PROMOTED.with(|entries| take_finished(&mut entries.borrow_mut(), request));
    for lease in leases.into_iter().flatten() {
        if crate::native_protocol::value(lease.popup, MANAGED) != lease.instance {
            if let Err(error) = restore(lease) {
                eprintln!("native-live-popup-layer-finish: {error}");
            }
        } // Hosts write MANAGED only after recovery-journal ACK; release still completes hide/destroy cleanup.
    }
}

#[cfg(test)]
#[path = "native_popup_layer_result_tests.rs"]
mod result_tests;
