//! 删除先同批撤下宿主与附属呈现，销毁和 GPU/任务回收不能留下单独一帧外框。
use super::*;

pub(in crate::desktop) unsafe fn hide_pin(host: HWND, state: &desktop::State) -> Result<()> {
    let mut handles = vec![host];
    if let Some(ui) = state.chrome.borrow().upgrade() {
        let node = ui
            .nodes
            .borrow()
            .iter()
            .find(|node| node.hwnd == host.0 as isize)
            .cloned();
        if let Some(node) = node {
            handles.extend(
                ui.borders
                    .borrow()
                    .iter()
                    .filter(|b| b.matches(&node))
                    .map(|b| b.hwnd),
            );
            handles.extend(
                ui.input_windows
                    .borrow()
                    .iter()
                    .filter(|(_, endpoint, _)| *endpoint == node.endpoint)
                    .map(|(hwnd, _, _)| *hwnd),
            );
            handles.extend(backing::moving(&ui, host));
            handles.extend(drawer::moving(&ui, host));
        }
    }
    handles.extend(desktop::pin_window::preview::handle(state));
    let mut rect = RECT::default();
    GetWindowRect(host, &mut rect)?;
    handles.extend(
        desktop::tool_panel::moving_positions(host, state, rect)?
            .into_iter()
            .map(|(hwnd, _)| hwnd),
    );
    handles.sort_unstable_by_key(|hwnd| hwnd.0 as usize);
    handles.dedup();
    handles.retain(|hwnd| IsWindowVisible(*hwnd).as_bool());
    if handles.is_empty() {
        return Ok(());
    }
    // 不跨 EndDeferWindowPos 持有 UI/Canvas 借用；同步通知可进入原几何与工具条 owner。
    let mut batch = BeginDeferWindowPos(handles.len() as i32)?;
    for hwnd in handles {
        batch = DeferWindowPos(
            batch,
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_HIDEWINDOW
                | SWP_NOMOVE
                | SWP_NOSIZE
                | SWP_NOZORDER
                | SWP_NOACTIVATE
                | SWP_NOOWNERZORDER,
        )?;
    }
    EndDeferWindowPos(batch)
}
