//! Tab 左右条与底栏紧邻所属钉图；公共曲线仍在全体钉图下，不改变图片层序或焦点。
use super::*;

struct Guard<'a>(&'a Cell<bool>);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
const FLAGS: SET_WINDOW_POS_FLAGS = SET_WINDOW_POS_FLAGS(
    SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOACTIVATE.0 | SWP_NOREDRAW.0 | SWP_NOOWNERZORDER.0,
);
unsafe fn topmost(hwnd: HWND) -> bool {
    GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0
}

pub(in super::super) unsafe fn apply(ui: &Ui) -> Result<()> {
    if ui.drawing.get().is_invalid() || ui.blocked() || ui.ordering.replace(true) {
        return Ok(());
    }
    // Win32 同步回调不得嵌套另一笔层序事务；跨系统调用不保留 RefCell 借用。
    let _guard = Guard(&ui.ordering);
    let nodes: Vec<_> = ui
        .nodes
        .borrow()
        .iter()
        .filter(|node| {
            let host = HWND(node.hwnd as *mut _);
            IsWindowVisible(host).as_bool() && GetWindowLongPtrW(host, GWLP_USERDATA) == node.marker
        })
        .cloned()
        .collect();
    let strips = ui.input_windows.borrow().clone();
    let backings = ui.backings.borrow().clone();
    let mut tails = HashSet::new();
    for node in &nodes {
        let host = HWND(node.hwnd as *mut _);
        let layers: Vec<_> = strips
            .iter()
            .filter(|(hwnd, endpoint, _)| {
                *endpoint == node.endpoint && IsWindow(Some(*hwnd)).as_bool()
            })
            .map(|(hwnd, _, _)| *hwnd)
            .chain(drawer::moving(ui, host))
            .chain(backings.iter().filter(|b| b.matches(node)).map(|b| b.hwnd))
            .collect();
        adjacent(host, &layers)?;
        tails.insert(layers.last().copied().unwrap_or(host).0 as isize);
    }
    if tails.is_empty() {
        return Ok(());
    }
    // 只有公共曲线保留全钉图下置策略；底栏属于自己的钉图，不属于公共线层。
    let layers = [ui.drawing.get()];
    let members: HashSet<_> = layers.iter().map(|hwnd| hwnd.0 as isize).collect();
    let mut cursor = GetTopWindow(None)?;
    let mut above = false;
    for _ in 0..4096 {
        if cursor.is_invalid() {
            break;
        }
        above |= members.contains(&(cursor.0 as isize));
        if tails.remove(&(cursor.0 as isize)) && tails.is_empty() {
            // 只纠正越过节点/Tab 的公共层；已合法下置的线层不能随刷新抬过外部应用。
            return if above {
                adjacent(cursor, &layers)
            } else {
                Ok(())
            };
        }
        cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
    }
    Ok(())
}

unsafe fn adjacent(anchor: HWND, layers: &[HWND]) -> Result<()> {
    let band = topmost(anchor);
    let mut previous = anchor;
    let mut changed = false;
    for &hwnd in layers {
        changed |= GetWindow(previous, GW_HWNDNEXT).ok() != Some(hwnd) || topmost(hwnd) != band;
        previous = hwnd;
    }
    if !changed || layers.is_empty() {
        return Ok(());
    }
    // 插在最后一个 TOPMOST 后不会自动变为 TOPMOST；必须先匹配 band。
    // 无 native owner 是刻意的：编辑 child 获取焦点不得顺带抬升图片。
    for &hwnd in layers {
        if topmost(hwnd) != band {
            SetWindowPos(
                hwnd,
                Some(if band { HWND_TOPMOST } else { HWND_NOTOPMOST }),
                0,
                0,
                0,
                0,
                FLAGS,
            )?;
        }
    }
    let mut batch = BeginDeferWindowPos(layers.len() as i32)?;
    // 固定锚点逆序插入，避免批次读取前一成员旧 band 导致后续成员再次升层。
    for &hwnd in layers.iter().rev() {
        batch = DeferWindowPos(batch, hwnd, Some(anchor), 0, 0, 0, 0, FLAGS)?;
    }
    EndDeferWindowPos(batch)
}
