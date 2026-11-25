//! Nested buttons may enter only current-transaction source popups/mirrored descendants; same process is not a provenance relationship.
use crate::{native_popup::Lease, native_protocol::Request};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

fn owned_by(mut child: HWND, parent: HWND) -> bool {
    if child == parent {
        return false;
    }
    unsafe {
        let mut process = 0;
        if GetWindowThreadProcessId(parent, Some(&mut process)) == 0 {
            return false;
        }
        for _ in 0..32 {
            let mut candidate_process = 0;
            if GetWindowThreadProcessId(child, Some(&mut candidate_process)) == 0
                || candidate_process != process
                || GetAncestor(child, GA_ROOT) != child
            {
                return false;
            }
            let Ok(owner) = GetWindow(child, GW_OWNER) else {
                return false;
            };
            if owner == parent {
                return true;
            }
            child = owner;
        }
    }
    false
}

pub(crate) fn allowed(parent: Request, child: Request) -> bool {
    if Request::read(parent.owner, parent.nonce, parent.destination) != Some(parent)
        || child.root == parent.root
        || !owned_by(child.owner, parent.owner)
    {
        return false;
    }
    owned_by(child.root, parent.root)
        || Lease::read(child.root).is_some_and(|lease| {
            lease.root == parent.root
                && lease.owner == parent.owner
                && lease.cookie == parent.cookie
                && lease.epoch == parent.epoch
        })
}
