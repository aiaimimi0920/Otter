//! 真实窗口上的应用热键调度；注册适配器替换为内存夹具，不占用系统快捷键。
use super::*;
use windows::core::Error;

struct Reset<'a>(&'a [DocumentWindow]);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        for document in self.0 {
            document.state.global_capture.set(false);
            document.state.hotkey_enabled.set(false);
            document.state.global_live_capture.set(false);
            document.state.global_long_capture.set(false);
            document.state.live_hotkey_enabled.set(false);
            document.state.long_hotkey_enabled.set(false);
        }
    }
}

unsafe fn checked(state: &State) -> Result<bool> {
    let menu = CreatePopupMenu()?;
    let result = (|| {
        super::super::global_capture::append(menu, state)?;
        Ok(GetMenuState(
            menu,
            super::super::global_capture::COMMAND as u32,
            MF_BYCOMMAND,
        ) & MF_CHECKED.0
            != 0)
    })();
    let _ = DestroyMenu(menu);
    result
}

pub(in crate::desktop) unsafe fn run(documents: &[DocumentWindow]) -> Result<()> {
    if documents.len() != 2
        || documents.iter().any(|document| {
            document.state.global_capture.get()
                || document.state.global_live_capture.get()
                || document.state.global_long_capture.get()
        })
    {
        return Err(Error::new(
            E_FAIL,
            "Hotkey smoke requires two unregistered fixture windows",
        ));
    }
    let _reset = Reset(documents);
    let first = &documents[0];
    let second = &documents[1];
    let mut calls = Vec::new();
    let mut failed = 0;
    let change = |_hwnd: HWND, state: &State, enabled: bool| {
        super::super::global_capture::update(state, enabled, |_, _| Ok(()))
    };
    request(first.window.0, &first.state)?;
    request(first.window.0, &first.state)?;
    process_with(
        documents,
        |hwnd, state, enabled| {
            calls.push((hwnd, enabled));
            change(hwnd, state, enabled)
        },
        |_, _| failed += 1,
    );
    if calls != [(first.window.0, true)]
        || failed != 0
        || !checked(&first.state)?
        || !checked(&second.state)?
        || !second.state.live_hotkey_enabled.get()
        || !second.state.long_hotkey_enabled.get()
        || !first.state.global_live_capture.get()
        || !first.state.global_long_capture.get()
    {
        return Err(Error::new(
            E_FAIL,
            "Application hotkey enable was not shared/coalesced",
        ));
    }
    // 未注册窗口的销毁清理是幂等空操作，不能把共享菜单状态改为关闭。
    super::super::global_capture::set(second.window.0, &second.state, false)?;
    if !checked(&second.state)? {
        return Err(Error::new(
            E_FAIL,
            "Non-owner cleanup cleared shared hotkey state",
        ));
    }
    request(second.window.0, &second.state)?;
    process_with(
        documents,
        |_, _, _| Err(Error::new(E_ACCESSDENIED, "fixture registration failure")),
        |_, _| failed += 1,
    );
    if failed != 1 || !first.state.global_capture.get() || !checked(&second.state)? {
        return Err(Error::new(
            E_FAIL,
            "Failed hotkey change lost registration/menu state",
        ));
    }
    request(second.window.0, &second.state)?;
    process_with(
        documents,
        |hwnd, state, enabled| {
            calls.push((hwnd, enabled));
            change(hwnd, state, enabled)
        },
        |_, _| failed += 1,
    );
    if calls.last() != Some(&(first.window.0, false))
        || checked(&first.state)?
        || checked(&second.state)?
    {
        return Err(Error::new(
            E_FAIL,
            "Other window could not disable the registered owner",
        ));
    }
    request(second.window.0, &second.state)?;
    process_with(documents, change, |_, _| failed += 1);
    if first.state.global_capture.get()
        || !second.state.global_capture.get()
        || first.state.global_live_capture.get()
        || first.state.global_long_capture.get()
        || !second.state.global_live_capture.get()
        || !second.state.global_long_capture.get()
        || !checked(&first.state)?
    {
        return Err(Error::new(
            E_FAIL,
            "Re-enable did not choose the requesting window",
        ));
    }
    request(first.window.0, &first.state)?;
    process_with(documents, change, |_, _| failed += 1);
    if failed != 1
        || first.state.live_hotkey_enabled.get()
        || first.state.long_hotkey_enabled.get()
        || documents.iter().any(|document| {
            document.state.global_capture.get()
                || document.state.global_live_capture.get()
                || document.state.global_long_capture.get()
        })
    {
        return Err(Error::new(
            E_FAIL,
            "Application hotkey smoke did not release fixture state",
        ));
    }
    println!("native-application-hotkey-smoke: shared-menu=true coalesced=true cross-window=true failure-preserved=true system-registration=false");
    println!("native-application-hotkey-secondary-smoke: live=true long=true unique-owner=true cleanup=true system-registration=false");
    Ok(())
}
