//! 真菜单模型/后台索引/键盘分页验证，不进入系统剪贴板或借用应用模型。
use super::*;
use windows::{
    core::Error,
    Win32::Foundation::{E_FAIL, HWND, POINT},
};
pub(in crate::desktop) fn select(hwnd: HWND, source: Source, library: usize) -> Result<Choice> {
    select_action(hwnd, source, library, false)
}
pub(in crate::desktop) fn select_action(
    hwnd: HWND,
    source: Source,
    library: usize,
    remove: bool,
) -> Result<Choice> {
    let mut tree = Vec::new();
    let mut model = LibraryMenu::attach(&mut tree, Some(source));
    let mut menu = Cascade::open(hwnd, tree, POINT { x: 30, y: 30 })?;
    model.update(&mut menu)?;
    for _ in 0..=library {
        menu.key(0x28)?;
    }
    menu.key(0x0d)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while model.data.is_none() {
        model.update(&mut menu)?;
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Frozen menu index timed out"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    if model
        .entries(library)
        .is_none_or(|entries| entries.len() != 65)
    {
        return Err(Error::new(E_FAIL, "Frozen menu fixture index mismatch"));
    }
    // 首页面板无焦点，向上选择最后一个启用项（下一页）。
    menu.key(0x26)?;
    menu.key(0x0d)?;
    if menu.closed() || menu.take_command().is_some() {
        return Err(Error::new(E_FAIL, "Page navigation closed menu"));
    }
    model.update(&mut menu)?;
    if model.pages[library] != 1 || !menu.submenu_open(model.roots[library]) {
        return Err(Error::new(E_FAIL, "Frozen second page not shown"));
    }
    menu.key(0x26)?;
    menu.key(0x0d)?;
    // Enter 展开时已聚焦首个动作，删除才再向下移动一次。
    if remove {
        menu.key(0x28)?;
    }
    menu.key(0x0d)?;
    if !menu.closed() {
        return Err(Error::new(E_FAIL, "Frozen leaf did not close menu"));
    }
    let choice = menu
        .take_command()
        .and_then(|command| model.take(command))
        .ok_or_else(|| Error::new(E_FAIL, "Frozen selection lost painted identity"))?;
    if choice.entry_id != "entry-64" || choice.remove != remove {
        return Err(Error::new(E_FAIL, "Frozen paging selected wrong identity"));
    }
    Ok(choice)
}
