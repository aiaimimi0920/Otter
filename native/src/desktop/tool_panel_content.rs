//! 隐藏钉图仅延迟 BUTTON；父面板/字体/截图保护 owner 不变，首次展示前完整投影。
use super::*;
const KEY: windows::core::PCWSTR = w!("Otter.ToolPanel.Content");
// 仅供 Win32 property 比较的非空标记，不代表资源句柄，也绝不解引用。
const BUILDING: HANDLE = HANDLE(std::ptr::without_provenance_mut(1));
const READY: HANDLE = HANDLE(std::ptr::without_provenance_mut(2));

pub(super) unsafe fn ready(panel: HWND) -> bool {
    GetPropW(panel, KEY) == READY
}
struct Pending(HWND, bool);
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.1 {
            unsafe {
                let _ = RemovePropW(self.0, KEY);
            }
        }
    }
}
pub(in crate::desktop) unsafe fn prepare(panel: HWND, state: &State) -> Result<()> {
    prepare_with(panel, state, || {
        children::create(panel, HINSTANCE(GetModuleHandleW(None)?.0))
    })
}
pub(super) unsafe fn prepare_with(
    panel: HWND,
    state: &State,
    create: impl FnOnce() -> Result<Vec<Window>>,
) -> Result<()> {
    if ready(panel) {
        return Ok(());
    }
    if !GetPropW(panel, KEY).is_invalid() {
        return Err(windows::core::Error::new(
            E_UNEXPECTED,
            "Tool panel content creation reentered",
        ));
    }
    SetPropW(panel, KEY, Some(BUILDING))?;
    let mut pending = Pending(panel, false);
    // 同步建窗回调可能重入，BUILDING 期间不刷新缓存；失败时先释放已建子窗口再清标记。
    let mut controls = crate::desktop::smoke_timing::run("tool-panel-controls", create)?;
    SetPropW(panel, KEY, Some(READY))?;
    for control in &mut controls {
        control.0 = HWND::default(); // 全部成功后交给父 HWND，之后不再执行可失败准备。
    }
    pending.1 = true;
    state.tool_panel_enabled.set(u64::MAX);
    state.tool_panel_tool.set(None);
    state.tool_panel_style.set(None);
    crate::desktop::smoke_timing::run("tool-panel-project", || {
        refresh(HWND(GetWindowLongPtrW(panel, GWLP_USERDATA) as *mut _));
        Ok::<_, windows::core::Error>(())
    })?;
    Ok(())
}
pub(super) unsafe fn before_show(panel: HWND, state: &State, position: &mut WINDOWPOS) {
    if position.flags.contains(SWP_SHOWWINDOW) && !position.flags.contains(SWP_HIDEWINDOW) {
        if let Err(error) = prepare(panel, state) {
            // 失败保持隐藏，临时控件已回滚；下一次展示可重试，不发布空/半完成的面板。
            position.flags &= !SWP_SHOWWINDOW;
            position.flags |= SWP_HIDEWINDOW;
            eprintln!("native-tool-panel-content: {error}");
        }
    }
}
