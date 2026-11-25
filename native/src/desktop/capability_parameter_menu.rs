//! Native shared scalar-parameter entry point; command indices refer only to the opening snapshot, never the latest catalog.
use super::parameter_action::Action;
use super::{configuration_stamp::Stamp, State};
#[path = "parameter_commit.rs"]
mod commit;
use serde_json::Value;
#[cfg(test)]
use windows::core::HSTRING;
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[cfg(test)]
const FIRST: usize = 0x7000;
const MAX_CHOICES: usize = 512;
fn edit_label(label: &str, value: &str, linked: bool) -> String {
    if !linked {
        return format!("{label}…");
    }
    let preview: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(64)
        .collect();
    crate::ui_text::format(
        "integration-linked-parameter",
        &[("label", label.into()), ("preview", preview.into())],
    )
}
fn reset_value(parameter: &crate::loom_capability::Parameter<'_>) -> Option<Option<Value>> {
    if parameter.secret {
        return None;
    }
    let fallback = match parameter.widget.as_str() {
        "number" | "slider" => Value::from(0),
        "checkbox" | "switch" => Value::Bool(false),
        "text" | "textarea" | "path" => Value::String(String::new()),
        "select" | "radio" => return Some(parameter.default.cloned()),
        "file" | "image_link" => return Some(Some(Value::String(String::new()))),
        _ => return None,
    };
    Some(Some(
        parameter
            .default
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(fallback),
    ))
}
#[cfg(test)]
pub(super) unsafe fn append(parent: HMENU, state: &State) -> Result<Option<Snapshot>> {
    let snapshot = Snapshot::capture(state);
    if let Some(snapshot) = &snapshot {
        snapshot.append(parent, state)?;
    }
    Ok(snapshot)
}
#[cfg(test)]
pub(super) unsafe fn apply(
    snapshot: Option<Snapshot>,
    hwnd: HWND,
    state: &State,
    command: usize,
) -> Result<bool> {
    let Some(snapshot) = snapshot.filter(|snapshot| snapshot.handles(command)) else {
        return Ok(false);
    };
    snapshot.select(hwnd, state, command)?;
    Ok(true)
}
pub(super) struct Choice {
    pub action: Action,
    pub id: String,
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
}
pub(super) struct Snapshot {
    view: Option<std::sync::Arc<()>>,
    stamp: Stamp,
    pub choices: Vec<Choice>,
}
#[cfg(test)]
struct Menu(HMENU);
#[cfg(test)]
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
impl Snapshot {
    pub fn capture(state: &State) -> Option<Self> {
        let selection = state.selected_art.borrow();
        let selection = selection.as_ref()?;
        let capability = selection
            .catalog
            .handshake
            .arts()
            .iter()
            .filter_map(crate::loom_capability::Capability::new)
            .find(|capability| capability.id == selection.parameters.art_id())?;
        let mut choices = Vec::new();
        let view = state.parameter_view.borrow();
        // Do not interpret a known stale view as "no edges": preserve identity so current() rejects commits until refreshed.
        let view = view.as_ref();
        for parameter in capability.parameters().filter(|p| !p.secret) {
            let Some(current) = selection.parameters.get(parameter.id) else {
                continue;
            };
            let linked = view.is_some_and(|view| view.linked.iter().any(|id| id == parameter.id));
            let effective = if linked {
                view.and_then(|view| view.values.get(parameter.id))
            } else {
                current
            };
            let display = super::parameter_display_value::effective(&parameter, effective);
            let enabled = !linked
                && capability.enabled
                && parameter.disabled != Some(true)
                && current.and_then(Value::as_str) != Some(crate::graph_params::DISABLED);
            if matches!(parameter.widget.as_str(), "checkbox" | "switch") {
                let checked = super::parameter_display_value::boolean(display);
                choices.push(Choice {
                    action: Action::Set(Value::Bool(!checked)),
                    id: parameter.id.into(),
                    label: parameter.label.into(),
                    checked,
                    enabled,
                });
            } else if matches!(parameter.widget.as_str(), "select" | "radio") {
                for option in parameter.options() {
                    if choices.len() == MAX_CHOICES {
                        break;
                    }
                    choices.push(Choice {
                        action: Action::Set(option.value.clone()),
                        id: parameter.id.into(),
                        label: format!("{}：{}", parameter.label, option.label),
                        checked: display == Some(option.value),
                        enabled,
                    });
                }
            } else if matches!(parameter.widget.as_str(), "file" | "image_link") {
                choices.push(Choice {
                    action: Action::File,
                    id: parameter.id.into(),
                    label: crate::ui_text::format(
                        "integration-choose-parameter-file",
                        &[("label", parameter.label.into())],
                    ),
                    checked: false,
                    enabled,
                });
            } else if parameter.widget == "color" {
                let initial = super::parameter_display_value::text(display);
                let label = edit_label(parameter.label, &initial, linked);
                choices.push(Choice {
                    action: Action::Color(initial),
                    id: parameter.id.into(),
                    label,
                    checked: false,
                    enabled,
                });
                if choices.len() < MAX_CHOICES {
                    choices.push(Choice {
                        action: Action::Set(Value::String(String::new())),
                        id: parameter.id.into(),
                        label: crate::ui_text::format(
                            "integration-clear-parameter-color",
                            &[("label", parameter.label.into())],
                        ),
                        checked: false,
                        enabled,
                    });
                }
            } else if let Some(spec) = super::parameter_value::Spec::from_parameter(&parameter) {
                if let Some(text) = spec.initial(display) {
                    let label = edit_label(parameter.label, &text, linked);
                    choices.push(Choice {
                        action: Action::Edit(spec, text),
                        id: parameter.id.into(),
                        label,
                        checked: false,
                        enabled,
                    });
                }
            }
            if choices.len() == MAX_CHOICES {
                break;
            }
        }
        for parameter in capability.parameters().filter(|p| !p.secret) {
            if choices.len() == MAX_CHOICES {
                break;
            }
            if selection.parameters.get(parameter.id).is_none() {
                continue;
            }
            let disabled = selection
                .parameters
                .get(parameter.id)
                .flatten()
                .and_then(Value::as_str)
                == Some(crate::graph_params::DISABLED);
            choices.push(Choice {
                action: Action::ToggleDisabled(
                    parameter
                        .default
                        .filter(|v| !v.is_null())
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                ),
                id: parameter.id.into(),
                label: format!(
                    "{}：{}",
                    parameter.label,
                    if disabled {
                        crate::ui_text::text("integration-enable-parameter")
                    } else {
                        crate::ui_text::text("integration-disable-parameter")
                    }
                ),
                checked: disabled,
                enabled: capability.enabled && parameter.disabled != Some(true),
            });
            if choices.len() == MAX_CHOICES {
                break;
            }
            let Some(value) = reset_value(&parameter) else {
                continue;
            };
            choices.push(Choice {
                action: Action::Reset(value),
                id: parameter.id.into(),
                label: format!(
                    "{}：{}",
                    parameter.label,
                    if matches!(parameter.widget.as_str(), "file" | "image_link") {
                        crate::ui_text::text("integration-clear-image")
                    } else {
                        crate::ui_text::text("integration-restore-default")
                    }
                ),
                checked: false,
                enabled: capability.enabled
                    && parameter.disabled != Some(true)
                    && !disabled
                    && !view.is_some_and(|view| view.linked.iter().any(|id| id == parameter.id)),
            });
        }
        if choices.is_empty() {
            return None;
        }
        Some(Self {
            view: view.map(|view| view.identity.clone()),
            stamp: Stamp::capture(state),
            choices,
        })
    }
    #[cfg(test)]
    pub unsafe fn append(&self, parent: HMENU, state: &State) -> Result<()> {
        let menu = Menu(CreatePopupMenu()?);
        for (index, choice) in self.choices.iter().enumerate() {
            let label: String = choice.label.chars().take(256).collect();
            AppendMenuW(
                menu.0,
                MF_STRING
                    | if choice.checked {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    }
                    | if choice.enabled && !state.group_locked.get() {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                FIRST + index,
                &HSTRING::from(label.replace('&', "&&")),
            )?;
        }
        AppendMenuW(
            parent,
            MF_POPUP,
            menu.0 .0 as usize,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-edit-parameters")),
        )?;
        std::mem::forget(menu);
        Ok(())
    }
    #[cfg(test)]
    pub fn handles(&self, command: usize) -> bool {
        command
            .checked_sub(FIRST)
            .is_some_and(|index| index < self.choices.len())
    }
    #[cfg(test)]
    pub unsafe fn select(&self, hwnd: HWND, state: &State, command: usize) -> Result<bool> {
        match command.checked_sub(FIRST) {
            Some(index) => self.submit(hwnd, state, index, None),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
#[path = "capability_parameter_menu_tests.rs"]
mod tests;
