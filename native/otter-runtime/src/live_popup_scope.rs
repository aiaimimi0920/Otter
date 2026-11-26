//! Local authorization for independent popups: same process does not mean same source; retain the owner chain to the user-authorized root.
use super::{identity, Target};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

const MAX_OWNER_DEPTH: usize = 32;
#[path = "live_popup_scope_transfer.rs"]
pub mod transfer;
pub use transfer::Transfer;
#[path = "live_popup_scope_native.rs"]
pub mod native;

/// Retain the original owner-chain channel; ownerless windows must hold that B's synchronous display lease.
#[derive(Clone)]
pub enum Scope {
    Owned(OwnedPopup),
    Chained {
        parent: Box<Scope>,
        child: Box<Scope>,
        owner: crate::live_hover_owner::Identity,
        epoch: usize,
    },
    Leased {
        root: Target,
        target: Target,
        anchor: Target,
        owner: crate::live_hover_owner::Identity,
        lease: otter_live_hover_hook::native_popup::Lease,
        descendant: Option<OwnedPopup>,
    },
}
impl Scope {
    /// Authorized attached sessions provide the next-level source; preserve display instances and B control epochs at every level, without retroactive PID-based authorization.
    pub fn bind_descendant(
        &self,
        owner: Option<crate::live_hover_owner::Identity>,
        hwnd: HWND,
    ) -> Result<Self> {
        let owner = owner.filter(|owner| owner.valid()).ok_or_else(denied)?;
        let epoch = otter_live_hover_hook::native_protocol::value(
            owner.window(),
            otter_live_hover_hook::native_protocol::EPOCH,
        );
        if epoch == 0 {
            return Err(denied());
        }
        let mut depth = 1;
        let mut ancestor = self;
        while let Self::Chained { parent, .. } = ancestor {
            depth += 1;
            ancestor = parent;
        }
        if depth >= 4 {
            return Err(denied());
        }
        let root = self.target()?;
        let child = Self::bind(&root, Some(owner), hwnd)?;
        Ok(Self::Chained {
            parent: Box::new(self.clone()),
            child: Box::new(child),
            owner,
            epoch,
        })
    }
    pub fn mirror_owner(&self) -> Option<HWND> {
        match self {
            Self::Chained { owner, .. } => Some(owner.window()),
            _ => None,
        }
    }
    pub fn layer_lease(&self) -> Option<otter_live_hover_hook::native_popup::Lease> {
        self.target().ok()?;
        match self {
            Self::Chained { child, .. } => child.layer_lease(),
            Self::Leased {
                lease,
                descendant: None,
                ..
            } => Some(*lease),
            _ => None,
        }
    }
    pub fn bind(
        root: &Target,
        owner: Option<crate::live_hover_owner::Identity>,
        hwnd: HWND,
    ) -> Result<Self> {
        // Fast-reject `root` itself and cross-process HWNDs before opening kernel process handles (`Target::bind`),
        // eliminating 25+ `OpenProcess`/`CloseHandle` pairs per second during idle 80ms popup discovery polls.
        if hwnd == root.hwnd() {
            return Err(denied());
        }
        let mut candidate_pid = 0u32;
        if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut candidate_pid)) } == 0
            || candidate_pid != root.process_id()
        {
            return Err(denied());
        }
        if let Ok(scope) = OwnedPopup::bind(root, hwnd) {
            return Ok(Self::Owned(scope));
        }
        let owner = owner.filter(|owner| owner.valid()).ok_or_else(denied)?;
        root.validate()?;
        let target = Target::bind(hwnd)?;
        if target.process_id() != root.process_id() || target.same_source(root) {
            return Err(denied());
        }
        let mut current = hwnd;
        for _ in 0..MAX_OWNER_DEPTH {
            if let Some(lease) = otter_live_hover_hook::native_popup::Lease::read(current) {
                if lease.root != root.hwnd()
                    || lease.owner != owner.window()
                    || lease.cookie != owner.cookie
                {
                    return Err(denied());
                }
                let anchor = Target::bind(current)?;
                let descendant = if current == hwnd {
                    None
                } else {
                    Some(OwnedPopup::bind(&anchor, hwnd)?)
                };
                let scope = Self::Leased {
                    root: root.clone(),
                    target,
                    anchor,
                    owner,
                    lease,
                    descendant,
                };
                scope.target()?;
                return Ok(scope);
            }
            current = unsafe { GetWindow(current, GW_OWNER)? };
        }
        Err(denied())
    }
    pub fn target(&self) -> Result<Target> {
        match self {
            Self::Owned(scope) => scope.target(),
            Self::Chained {
                parent,
                child,
                owner,
                epoch,
            } => {
                parent.target()?;
                if !owner.valid()
                    || otter_live_hover_hook::native_protocol::value(
                        owner.window(),
                        otter_live_hover_hook::native_protocol::EPOCH,
                    ) != *epoch
                {
                    return Err(denied());
                }
                child.target()
            }
            Self::Leased {
                root,
                target,
                anchor,
                owner,
                lease,
                descendant,
            } => {
                root.validate()?;
                target.validate()?;
                anchor.validate()?;
                if !owner.valid() || !lease.valid() {
                    return Err(denied());
                }
                if let Some(descendant) = descendant {
                    descendant.validate()?;
                }
                Ok(target.clone())
            }
        }
    }
}

/// Save original process identities for root and popup; revalidate ownership before every use, without treating bare HWNDs as long-term authorization.
#[derive(Clone)]
pub struct OwnedPopup {
    root: Target,
    popup: Target,
}

impl OwnedPopup {
    pub fn bind(root: &Target, hwnd: HWND) -> Result<Self> {
        if hwnd == root.hwnd() {
            return Err(denied());
        }
        let mut candidate_pid = 0u32;
        if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut candidate_pid)) } == 0
            || candidate_pid != root.process_id()
        {
            return Err(denied());
        }
        root.validate()?;
        let popup = Target::bind(hwnd)?;
        let binding = Self {
            root: root.clone(),
            popup,
        };
        binding.validate()?;
        Ok(binding)
    }

    pub fn validate(&self) -> Result<()> {
        self.root.validate()?;
        self.popup.validate()?;
        if self.root.same_source(&self.popup) || self.root.process_id() != self.popup.process_id() {
            return Err(denied());
        }
        let mut current = self.popup.hwnd();
        let mut visited = [0usize; MAX_OWNER_DEPTH];
        for depth in 0..MAX_OWNER_DEPTH {
            let value = current.0 as usize;
            if visited[..depth].contains(&value) {
                return Err(denied());
            }
            visited[depth] = value;
            // Owner chains allow only same-process top-level windows; never infer ownership from titles, class names, or geometric overlap.
            let current_identity = unsafe { identity(current)? };
            if current_identity.process != self.root.process_id() {
                return Err(denied());
            }
            let owner = unsafe { GetWindow(current, GW_OWNER)? };
            if owner == self.root.hwnd() {
                self.root.validate()?;
                self.popup.validate()?;
                return Ok(());
            }
            if owner.0.is_null() {
                return Err(denied());
            }
            current = owner;
        }
        Err(denied())
    }

    /// Get the currently validated target for capture/input binding; holders must still call validate on lifecycle changes.
    pub fn target(&self) -> Result<Target> {
        self.validate()?;
        Ok(self.popup.clone())
    }
}

fn denied() -> Error {
    Error::new(
        E_ACCESSDENIED,
        "Live popup is not owned by the authorized source",
    )
}

#[cfg(test)]
#[path = "live_popup_scope_tests.rs"]
mod tests;
