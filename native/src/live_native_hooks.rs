//! 同包 DLL 的定向 hook owner；输入与独立恢复共用加载/回滚/卸载，永不安装线程 0。
use crate::live_window_target::Target;
use std::os::windows::ffi::OsStrExt;
use windows::{
    core::{s, Error, Result, PCWSTR},
    Win32::{Foundation::*, System::LibraryLoader::*, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Hover,
    Native,
    Layer,
}
pub(crate) struct Hooks {
    hooks: Vec<HHOOK>,
    module: HMODULE,
}
impl Hooks {
    pub(crate) fn install(target: &Target, kind: Kind) -> Result<Self> {
        target.validate()?;
        let thread = unsafe { GetWindowThreadProcessId(target.hwnd(), None) };
        if thread == 0 {
            return Err(Error::new(E_ABORT, "Live source thread closed"));
        }
        target.validate()?;
        #[cfg(test)]
        if kind == Kind::Layer
            && target.process_id()
                == unsafe { windows::Win32::System::Threading::GetCurrentProcessId() }
        {
            return Self::local_layer(target, thread);
        }
        let path = std::env::current_exe()
            .map_err(|e| Error::new(E_FAIL, e.to_string()))?
            .parent()
            .ok_or_else(|| Error::new(E_FAIL, "Live executable directory missing"))?
            .join("otter_live_hover_hook.dll");
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            let module = LoadLibraryExW(
                PCWSTR(wide.as_ptr()),
                None,
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            )?;
            let mut owner = Self {
                hooks: Vec::new(),
                module,
            };
            let exports = match kind {
                Kind::Layer => [
                    (WH_GETMESSAGE, s!("otter_live_layer_get")),
                    (WH_CALLWNDPROC, s!("otter_live_layer_call")),
                    (WH_CALLWNDPROCRET, s!("otter_live_native_return")),
                ],
                _ => [
                    (WH_GETMESSAGE, s!("otter_live_hover_hook")),
                    (WH_CALLWNDPROC, s!("otter_live_native_call")),
                    (WH_CALLWNDPROCRET, s!("otter_live_native_return")),
                ],
            };
            for (kind_id, name) in exports
                .into_iter()
                .take(if kind == Kind::Hover { 1 } else { 3 })
            {
                target.validate()?;
                let proc = GetProcAddress(module, name)
                    .ok_or_else(|| Error::new(E_FAIL, "Live hook export missing"))?;
                owner.hooks.push(SetWindowsHookExW(
                    kind_id,
                    Some(std::mem::transmute::<
                        unsafe extern "system" fn() -> isize,
                        unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT,
                    >(proc)),
                    Some(HINSTANCE(module.0)),
                    thread,
                )?);
            }
            target.validate()?;
            Ok(owner) // 任何部分安装失败都由同一个 Drop 回滚，不能丢本地 DLL 引用。
        }
    }
    #[cfg(test)]
    fn local_layer(target: &Target, thread: u32) -> Result<Self> {
        // 仅本测试进程的自有窗口使用 rlib 回调；跨进程仍强制同包实际 DLL。
        let mut owner = Self {
            hooks: Vec::new(),
            module: HMODULE::default(),
        };
        for (kind, callback) in [
            (
                WH_GETMESSAGE,
                otter_live_hover_hook::native_layer_dispatch::otter_live_layer_get
                    as unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT,
            ),
            (
                WH_CALLWNDPROC,
                otter_live_hover_hook::native_layer_dispatch::otter_live_layer_call,
            ),
            (
                WH_CALLWNDPROCRET,
                otter_live_hover_hook::native_popup::otter_live_native_return,
            ),
        ] {
            target.validate()?;
            owner
                .hooks
                .push(unsafe { SetWindowsHookExW(kind, Some(callback), None, thread)? });
        }
        target.validate()?;
        Ok(owner)
    }
    pub(crate) fn closed(&self) -> bool {
        self.hooks.is_empty() && self.module.is_invalid()
    }
    pub(crate) fn detach(&mut self) -> Result<()> {
        unsafe {
            while let Some(&hook) = self.hooks.last() {
                if let Err(error) = UnhookWindowsHookEx(hook) {
                    if error.code() != ERROR_INVALID_HOOK_HANDLE.to_hresult() {
                        return Err(error); // 未确认卸载，不释放仍可能执行回调的模块。
                    }
                }
                self.hooks.pop();
            }
            if !self.module.is_invalid() {
                FreeLibrary(self.module)?;
                self.module = HMODULE::default();
            }
        }
        Ok(())
    }
}
impl Drop for Hooks {
    fn drop(&mut self) {
        if let Err(error) = self.detach() {
            eprintln!("native-live-hook-cleanup: {error}");
        }
    }
}
