//! 字体弹层只提交原目标的 family；枚举/排序归应用，滚动表现归 Neuro-mini。
use super::*;
fn current(c: &Canvas) -> Option<String> {
    let v = fields::view(c)?;
    Some(
        v.selected
            .and_then(|i| c.editor.elements()[i].text.as_ref())
            .map(|t| t.family())
            .unwrap_or_else(|| {
                if v.tool == crate::editor::Tool::Serial {
                    c.tool_profiles.serial_font_family()
                } else {
                    c.tool_profiles.text_font.family()
                }
            })
            .to_owned(),
    )
}
pub(super) unsafe fn execute(
    owner: HWND,
    state: &State,
    anchor: RECT,
    target: &MenuTarget,
) -> Result<()> {
    let Some(current) = current(&state.canvas.borrow()) else {
        return Ok(());
    };
    // 懒加载成功结果后复用；读取名称不加载字体文件，也不弹出另一层系统字体框。
    let fonts = super::super::installed_fonts::list()?;
    let selected = fonts.iter().position(|name| *name == current);
    let result = {
        let Some(_modal) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(());
        };
        let panel = state.tool_panel.get();
        neuro_mini_ui::set_property_expanded(panel, FONT, true)?;
        let result = neuro_mini_ui::track_list(panel, anchor, fonts.to_vec(), selected);
        if neuro_mini_ui::is_toolbar(panel) {
            neuro_mini_ui::set_property_expanded(panel, FONT, false)?;
        }
        result?
    };
    if let Some(index) = result {
        if menus::valid_target(owner, state, target) {
            apply::font(&mut state.canvas.borrow_mut(), &fonts[index], None)?;
        }
    }
    Ok(())
}
