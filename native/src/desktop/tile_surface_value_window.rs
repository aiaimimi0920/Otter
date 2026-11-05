//! Tile离散值手势与原生选项菜单；不持COM/Shared借用跨菜单泵，取消不发动作。
use super::*;
use crate::tile_surface_value::{Control, Field};
pub(in crate::desktop::tile_window) fn field(state: &State, pixel: [i32; 2]) -> Option<Field> {
    let frame = active(state)?;
    let mut field = state
        .renderer
        .borrow()
        .as_ref()?
        .value(&frame, pixel, modifiers())?;
    match super::super::local::focus(state, pixel)
        .and_then(|focus| super::super::local::value(state, &focus))
    {
        Some(crate::tile_surface_local::Value::Switch(value)) => {
            field.control = Control::Switch(value)
        }
        Some(crate::tile_surface_local::Value::Select(value)) => field.selection = value,
        _ => {}
    }
    Some(field)
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
fn menu(options: &crate::surface_select::Options, selected: Option<usize>) -> Result<Menu> {
    let menu = Menu(unsafe { CreatePopupMenu()? });
    for (index, option) in options.items().iter().enumerate() {
        // Win32菜单解释&和tab；远端声明只能成为文字，不能注入快捷键/列或额外项。
        let label = option
            .label()
            .replace('&', "&&")
            .replace(['\r', '\n', '\t'], " ");
        unsafe {
            AppendMenuW(
                menu.0,
                MF_STRING
                    | if Some(index) == selected {
                        MF_CHECKED
                    } else {
                        MENU_ITEM_FLAGS(0)
                    },
                index + 1,
                &windows::core::HSTRING::from(label),
            )?;
        }
    }
    Ok(menu)
}
pub(super) fn choose(hwnd: HWND, field: &Field) -> Result<Option<usize>> {
    let Control::Select(options) = &field.control else {
        return Ok(None);
    };
    if options.items().is_empty() {
        return Ok(None);
    }
    let menu = menu(options, field.selection)?;
    let mut point = POINT {
        x: field.anchor.pixel[0],
        y: field.anchor.pixel[1],
    };
    unsafe {
        if !ClientToScreen(hwnd, &mut point).as_bool() {
            return Err(Error::from_win32());
        }
        let index = TrackPopupMenu(
            menu.0,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        )
        .0 as usize;
        Ok(index.checked_sub(1).filter(|i| *i < options.items().len()))
    }
}
pub(in crate::desktop::tile_window) fn finish(
    hwnd: HWND,
    state: &State,
    mut prior: Field,
    pixel: [i32; 2],
    chooser: impl FnOnce(&Field) -> Result<Option<usize>>,
) {
    if field(state, pixel).is_none_or(|current| !prior.same(&current)) {
        return;
    }
    let local_focus = super::super::local::focus(state, pixel);
    let local_value = local_focus
        .as_ref()
        .and_then(|f| super::super::local::value(state, f));
    let selected = match prior.control {
        Control::Select(_) => match chooser(&prior) {
            Ok(Some(index)) => Some(index),
            Ok(None) => return,
            Err(_) => {
                state.shared.lock().actions.notice = "Art选项菜单不可用";
                return;
            }
        },
        Control::Switch(_) => None,
    };
    // 模态菜单期间可发生撤权、黑场、换版、显示/DPI变化或同步销毁。
    if state.closed.get()
        || state.changed.get()
        || !unsafe { IsWindowVisible(hwnd) }.as_bool()
        || field(state, pixel).is_none_or(|current| !prior.same(&current))
    {
        return;
    }
    prior.anchor.pixel = pixel;
    let changed = match local_value {
        Some(crate::tile_surface_local::Value::Switch(value)) => {
            Some(crate::tile_surface_local::Value::Switch(!value))
        }
        Some(crate::tile_surface_local::Value::Select(_)) => {
            Some(crate::tile_surface_local::Value::Select(selected))
        }
        _ => None,
    };
    let dependency = (
        prior.anchor.view.clone(),
        prior.node.clone(),
        prior.anchor.snapshot.clone(),
    );
    let mut commands = match prior.commands(selected) {
        Ok(commands) => commands,
        Err(error) => {
            state.shared.lock().actions.notice = error;
            return;
        }
    };
    if let Ok(scene) = crate::surface_scene::Scene::parse(&dependency.2.scene) {
        if let Some(node) = scene.root().visible_node(&dependency.1) {
            if let Ok(contract) = crate::surface_edit_contract::fingerprint(node) {
                if let Some(edit) =
                    super::super::text::peer(state, &dependency.0, dependency.1, contract)
                {
                    for command in &mut commands {
                        command.edit = Some(edit.clone());
                    }
                }
            }
        }
    }
    if super::super::text::decorate(state, &mut commands).is_err() {
        state.shared.lock().actions.notice = "Art表单未提交";
        return;
    }
    if let Some((focus, value)) = local_focus.zip(changed) {
        if super::super::local::set(state, focus, value).is_err() {
            state.shared.lock().actions.notice = "Art本地字段未修改";
            return;
        }
    }
    // 无跨UI调度借用；worker还将GET新鲜state并重新核节点/命中后才发送一次。
    if !commands.is_empty() {
        state.shared.lock().actions.offer(commands, Instant::now());
    }
}
#[cfg(test)]
pub(super) fn test_value(
    hwnd: HWND,
    state: &State,
    pixel: [i32; 2],
    selected: Option<usize>,
) -> bool {
    let before = state.shared.lock().actions.test_offered;
    down(state, pixel);
    let prior = std::mem::take(&mut *state.pressed.borrow_mut());
    if let Gesture::Value(field) = prior {
        finish(hwnd, state, *field, pixel, |_| Ok(selected));
    }
    state.shared.lock().actions.test_offered > before
}
#[cfg(test)]
#[path = "tile_surface_value_window_tests.rs"]
mod tests;
