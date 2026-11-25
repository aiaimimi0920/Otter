//! Original popup-provenance value snapshots for exact input; at most four segments, without reacquiring authorization from current leases.
use crate::{native_popup::Lease, native_protocol, scope};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

#[path = "native_input_scope_wire.rs"]
mod wire;
pub use wire::keys;
pub const LIMIT: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Link {
    pub root: usize,
    pub root_thread: usize,
    pub target: usize,
    pub target_thread: usize,
    pub owner: usize,
    pub cookie: usize,
    pub epoch: usize,
    pub anchor: usize,
    pub anchor_thread: usize,
    pub request: usize,
    pub instance: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    links: [Link; LIMIT],
    len: usize,
}
impl Snapshot {
    pub fn push(&mut self, link: Link) -> bool {
        if self.len == LIMIT {
            return false;
        }
        self.links[self.len] = link;
        self.len += 1;
        true
    }
    /// Used for host pre-submit rechecks and source-thread read/claim checks; process permissions remain with original Destination.
    pub fn valid_for(self, root: HWND, owner: HWND) -> bool {
        if self.len == 0 {
            return true;
        }
        let mut source_process = 0;
        let mut host_process = 0;
        unsafe {
            GetWindowThreadProcessId(root, Some(&mut source_process));
            GetWindowThreadProcessId(owner, Some(&mut host_process));
        }
        if source_process == 0 || host_process == 0 {
            return false;
        }
        let links = &self.links[..self.len];
        links
            .last()
            .is_some_and(|link| link.target == root.0 as usize)
            && unsafe { IsWindowVisible(root).as_bool() && !IsIconic(root).as_bool() }
            && links.iter().enumerate().all(|(index, link)| {
                (index == 0
                    || (link.root == links[index - 1].target
                        && link.root_thread == links[index - 1].target_thread
                        && link.owner != 0))
                    && link.valid(source_process, host_process)
            })
    }
}

fn source_window(hwnd: usize, thread: usize, process: u32) -> bool {
    let hwnd = HWND(hwnd as *mut _);
    let mut actual_process = 0;
    let actual_thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut actual_process)) };
    actual_thread != 0
        && actual_thread as usize == thread
        && actual_process == process
        && unsafe { GetAncestor(hwnd, GA_ROOT) == hwnd }
}

pub(crate) fn owned_by(root: usize, mut target: usize, process: u32) -> bool {
    if root == target {
        return false;
    }
    let mut visited = [0usize; 32];
    for depth in 0..visited.len() {
        if visited[..depth].contains(&target) {
            return false;
        }
        visited[depth] = target;
        let hwnd = HWND(target as *mut _);
        let mut actual_process = 0;
        unsafe {
            if GetWindowThreadProcessId(hwnd, Some(&mut actual_process)) == 0
                || actual_process != process
                || GetAncestor(hwnd, GA_ROOT) != hwnd
            {
                return false;
            }
        }
        let Ok(owner) = (unsafe { GetWindow(hwnd, GW_OWNER) }) else {
            return false;
        };
        if owner.0 as usize == root {
            return true;
        }
        target = owner.0 as usize;
    }
    false
}

impl Link {
    fn valid(self, source_process: u32, host_process: u32) -> bool {
        if !source_window(self.root, self.root_thread, source_process)
            || !source_window(self.target, self.target_thread, source_process)
            || self.root == self.target
        {
            return false;
        }
        if self.owner == 0 {
            if self.cookie != 0 || self.epoch != 0 || self.anchor != 0 {
                return false;
            }
        } else {
            let owner = HWND(self.owner as *mut _);
            let mut process = 0;
            unsafe { GetWindowThreadProcessId(owner, Some(&mut process)) };
            if process != host_process
                || self.cookie == 0
                || self.epoch == 0
                || native_protocol::value(owner, scope::COOKIE) != self.cookie
                || native_protocol::value(owner, native_protocol::EPOCH) != self.epoch
            {
                return false;
            }
        }
        if self.anchor == 0 {
            return self.anchor_thread == 0
                && self.request == 0
                && self.instance == 0
                && owned_by(self.root, self.target, source_process);
        }
        let lease = Lease {
            popup: HWND(self.anchor as *mut _),
            root: HWND(self.root as *mut _),
            owner: HWND(self.owner as *mut _),
            cookie: self.cookie,
            epoch: self.epoch,
            request: self.request,
            instance: self.instance,
        };
        source_window(self.anchor, self.anchor_thread, source_process)
            && lease.valid()
            && (self.anchor == self.target || owned_by(self.anchor, self.target, source_process))
    }
}
