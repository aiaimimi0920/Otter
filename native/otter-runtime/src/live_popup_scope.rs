//! 独立弹窗的局部授权：同进程不等于同源，必须保留到用户授权 root 的 owner 链。
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

/// 保留原 owner 链通道；无 owner 窗口必须持有该 B 的同步显示租约。
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
    /// 已授权附属会话提供下一层来源；仍逐层保留显示实例和 B 控制周期，不按 PID 追认。
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

/// 保存根和弹窗的原进程身份；每次使用前重验归属，不能将裸 HWND 当作长期授权。
#[derive(Clone)]
pub struct OwnedPopup {
    root: Target,
    popup: Target,
}

impl OwnedPopup {
    pub fn bind(root: &Target, hwnd: HWND) -> Result<Self> {
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
            // owner 链只允许同进程的顶层窗口；不以标题、类名、几何重叠猜测归属。
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

    /// 为捕获/输入绑定取得当前已验证目标；持有者仍须在生命周期变化时调用 validate。
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
