//! Ctrl+E 的通用扩展工具入口；复用目录、单命令 owner 与模态租约，不另建执行器或资源权限。
use super::{
    extension_command_route::Context, extension_document, tool_panel_availability::Availability,
    State,
};
use crate::loom_extension_snapshot::Snapshot;
use std::{rc::Rc, sync::Arc};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "edit_extension_toolbar_model.rs"]
mod model;
use model::Model;
pub(super) const FIRST: u16 = 0x6300;
#[derive(Default)]
pub(super) struct Session {
    model: Option<Rc<Model>>,
    token: u32,
}
pub(super) fn handles(command: usize) -> bool {
    (FIRST..FIRST + 8).contains(&(command as u16))
}
unsafe fn capture(state: &State) -> Result<Option<Rc<Model>>> {
    extension_document::observe(state);
    let cached = state.extension_toolbar.borrow().model.clone();
    if let Some(model) = cached.filter(|m| m.current(state)) {
        return Ok(Some(model));
    }
    let catalog = state.extensions.borrow().snapshot.clone();
    let mut session = state.extension_toolbar.borrow_mut();
    session.model = None;
    let Some(catalog) = catalog else {
        return Ok(None);
    };
    session.token = session
        .token
        .checked_add(1)
        .ok_or_else(|| Error::from(E_BOUNDS))?;
    let model = Rc::new(Model::capture(state, catalog, session.token)?);
    session.model = Some(model.clone());
    Ok(Some(model))
}
fn enabled(state: &State) -> bool {
    enabled_with(state, Availability::capture(state))
}
fn enabled_with(state: &State, available: Option<Availability>) -> bool {
    !state.session_end.paused()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && super::capture_input::exclusive_available(state)
        && available.is_some_and(|a| a.allows(0x53) && a.allows(super::context_menu::COLOR))
}
unsafe fn project(
    state: &State,
    panel: HWND,
    model: Option<&Model>,
    enabled: bool,
    open: Option<usize>,
) -> Result<()> {
    let labels: Vec<_> = model
        .into_iter()
        .flat_map(|m| &m.slots)
        .map(|slot| {
            let title = if !slot.grouped && !slot.items[0].available {
                slot.items[0]
                    .disabled_title
                    .as_deref()
                    .unwrap_or(&slot.title)
            } else {
                &slot.title
            };
            (model::text(&slot.title, 64), model::text(title, 384))
        })
        .collect();
    let mut slots = Vec::new();
    if let Some(m) = model {
        for (index, slot) in m.slots.iter().enumerate() {
            slots.push(neuro_mini_ui::TextSlot {
                id: FIRST + index as u16,
                text: &labels[index].0,
                label: &labels[index].1,
                dropdown: slot.grouped,
                active: open == Some(index),
                enabled: enabled
                    && (slot.grouped
                        || slot.items[0].available
                            && super::extension_commands::command_available(
                                state,
                                &slot.items[0].command,
                            )),
                context: m.token,
            });
        }
    }
    neuro_mini_ui::set_text_slots(panel, &slots)
}
pub(super) unsafe fn refresh(state: &State) -> Result<()> {
    let panel = state.tool_panel.get();
    match capture(state) {
        Ok(model) => project(
            state,
            panel,
            model.as_deref(),
            enabled_with(
                state,
                Availability::capture(state).map(Availability::presentation),
            ),
            None,
        ),
        Err(error) => {
            project(state, panel, None, false, None)?;
            Err(error)
        }
    }
}
pub(super) unsafe fn execute(owner: HWND, state: &State, command: usize) -> Result<()> {
    execute_with(owner, state, command, |panel, anchor, options| {
        show_options(panel, anchor, options)
    })
}
unsafe fn show_options(
    panel: HWND,
    anchor: RECT,
    options: Vec<neuro_mini_ui::ListOption>,
) -> Result<Option<usize>> {
    if options.len() <= 12 && options.iter().all(|i| i.label.len() <= 256) {
        let menu = options
            .iter()
            .enumerate()
            .map(|(index, item)| neuro_mini_ui::MenuItem {
                id: index as u16 + 1,
                label: item.label.clone(),
                shortcut: String::new(),
                icon: neuro_mini_ui::Icon::stroke(""),
                selected: false,
                enabled: item.enabled,
            })
            .collect();
        match neuro_mini_ui::track_menu(panel, anchor, menu, true) {
            Ok(selected) => return Ok(selected.map(|i| i as usize - 1)),
            Err(error) if error.code() == E_BOUNDS => (),
            Err(error) => return Err(error),
        }
    }
    // 长菜单或工作区不足时保留全部条目，使用同主题固定 BUTTON 池的局部滚动，而非截断。
    neuro_mini_ui::track_options(panel, anchor, options)
}
pub(super) unsafe fn execute_with(
    owner: HWND,
    state: &State,
    command: usize,
    show: impl FnOnce(HWND, RECT, Vec<neuro_mini_ui::ListOption>) -> Result<Option<usize>>,
) -> Result<()> {
    if !state.lifetime.owns(owner) || !enabled(state) {
        return Ok(());
    }
    extension_document::observe(state);
    let model = state.extension_toolbar.borrow().model.clone();
    let Some(model) = model.filter(|m| m.token as usize == command >> 32 && m.current(state))
    else {
        return Ok(());
    };
    let index = (command as u16 - FIRST) as usize;
    let Some(slot) = model.slots.get(index) else {
        return Ok(());
    };
    let panel = state.tool_panel.get();
    let selected = if slot.grouped {
        project(state, panel, Some(&model), true, Some(index))?;
        let result = super::tool_panel::popup::preserve(state, || {
            let Some(_modal) = super::file_dialog::Guard::begin(&state.file_dialog) else {
                return Ok(None);
            };
            let mut anchor = RECT::default();
            GetWindowRect(
                GetDlgItem(Some(panel), (FIRST + index as u16) as i32)?,
                &mut anchor,
            )?;
            let options = slot
                .items
                .iter()
                .map(|i| neuro_mini_ui::ListOption {
                    label: model::text(&i.title, 384),
                    enabled: i.available
                        && super::extension_commands::command_available(state, &i.command),
                })
                .collect();
            show(panel, anchor, options)
        });
        if state.lifetime.owns(owner) && state.tool_panel.get() == panel {
            refresh(state)?;
        }
        result?
    } else {
        Some(0)
    };
    if !state.lifetime.owns(owner)
        || state.tool_panel.get() != panel
        || !enabled(state)
        || !model.current(state)
    {
        return Ok(());
    }
    if let Some(item) = selected
        .and_then(|i| slot.items.get(i))
        .filter(|i| i.available && super::extension_commands::command_available(state, &i.command))
    {
        super::extension_commands::request_command(owner, state, &item.command)?;
    }
    Ok(())
}
