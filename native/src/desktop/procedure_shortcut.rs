//! Resolve a canvas chord to a stable menu command, after inline edit and modal dispatch.
use super::State;
use crate::shortcut_policy::{Chord, Policy, ALT, CANVAS, CTRL, SHIFT};
use windows::Win32::{Foundation::*, UI::Input::KeyboardAndMouse::*};

fn resolve(policy: &Policy, chord: Chord) -> Option<usize> {
    CANVAS
        .iter()
        .find(|(id, _)| policy.chord(id) == Some(chord))
        .and_then(|(id, _)| crate::shortcut_policy::canvas_command(id))
}
pub(super) unsafe fn tool(hwnd: HWND, state: &State, key: usize) -> bool {
    if GetKeyState(VK_LWIN.0.into()) < 0 || GetKeyState(VK_RWIN.0.into()) < 0 {
        return false;
    }
    let Ok(key) = u8::try_from(key) else {
        return false;
    };
    let modifiers = [(VK_MENU, ALT), (VK_CONTROL, CTRL), (VK_SHIFT, SHIFT)]
        .iter()
        .fold(0, |bits, (vk, bit)| {
            if GetKeyState(vk.0.into()) < 0 {
                bits | bit
            } else {
                bits
            }
        });
    let Some(id) = resolve(&state.shortcut_policy.borrow(), Chord { key, modifiers }) else {
        return false;
    };
    if !state.modals.blocked()
        && !state.session_end.paused()
        && !state.group_hidden.get()
        && super::tool_panel_availability::Availability::capture(state)
            .is_some_and(|a| a.allows(id))
    {
        if let Err(error) = super::context_menu::execute(hwnd, state, id) {
            eprintln!("native-tool-shortcut: {error}");
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut_policy::Binding;
    #[test]
    fn remapping_does_not_change_menu_identity_or_leave_an_old_alias() {
        let p = Policy {
            bindings: vec![Binding {
                action: "tool.blur".into(),
                chord: Some(Chord {
                    key: b'9',
                    modifiers: SHIFT,
                }),
            }],
            ..Default::default()
        };
        assert!(p.validate().is_ok());
        assert_eq!(
            resolve(
                &p,
                Chord {
                    key: b'G',
                    modifiers: 0
                }
            ),
            None
        );
        assert_eq!(
            resolve(
                &p,
                Chord {
                    key: b'9',
                    modifiers: SHIFT
                }
            ),
            Some(0x47)
        );
        for (id, _) in CANVAS {
            let entry = crate::desktop::tool_commands::TOOLS
                .iter()
                .find(|entry| Some(entry.id) == crate::shortcut_policy::canvas_command(id))
                .unwrap();
            assert_eq!(entry.label_id.replace('-', "."), *id);
        }
    }
}
