//! 非激活菜单期间，owner 同步收到的滚轮也交给菜单；随模态作用域撤销子类。
use super::*;
const WHEEL_ID: usize = 0x4f4d57;
struct Target {
    root: HWND,
    identity: usize,
}
pub(crate) struct OwnerWheel {
    owner: HWND,
    target: Box<Target>,
}
impl Panel {
    pub(crate) fn forward_owner_wheel(&self, owner: HWND) -> Result<OwnerWheel> {
        let guard = OwnerWheel {
            owner,
            target: Box::new(Target {
                root: self.handle()?,
                identity: (&*self.data as *const Data) as usize,
            }),
        };
        unsafe {
            if !SetWindowSubclass(
                owner,
                Some(procedure),
                WHEEL_ID,
                (&*guard.target as *const Target) as usize,
            )
            .as_bool()
            {
                return Err(Error::from_win32());
            }
        }
        Ok(guard)
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    pointer: usize,
) -> LRESULT {
    if message == WM_MOUSEWHEEL {
        let target = &*(pointer as *const Target);
        let mut identity = 0;
        if GetWindowSubclass(target.root, Some(input::procedure), ID, Some(&mut identity)).as_bool()
            && identity == target.identity
        {
            return SendMessageW(target.root, message, Some(wp), Some(lp));
        }
    }
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), WHEEL_ID);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
impl Drop for OwnerWheel {
    fn drop(&mut self) {
        unsafe {
            let mut pointer = 0;
            if GetWindowSubclass(self.owner, Some(procedure), WHEEL_ID, Some(&mut pointer))
                .as_bool()
                && pointer == (&*self.target as *const Target) as usize
            {
                let _ = RemoveWindowSubclass(self.owner, Some(procedure), WHEEL_ID);
            }
        }
    }
}
