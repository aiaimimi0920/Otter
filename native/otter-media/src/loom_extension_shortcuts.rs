//! Mainline plugin shortcut declarations mapped to bounded native keys; only projects a trusted catalog, without registering hotkeys or granting resource access.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Chord {
    pub key: u32,
    /// Stable Win32 MOD_ALT/CONTROL/SHIFT/WIN bit values; excludes runtime MOD_NOREPEAT.
    pub modifiers: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub id: String,
    pub command: String,
    pub scope: String,
    pub chord: Chord,
    pub global: bool,
}
fn lowercase_key(value: &str) -> Option<u32> {
    Some(match value {
        "esc" | "escape" => 0x1b,
        "delete" | "del" => 0x2e,
        "backspace" => 8,
        "tab" => 9,
        "space" | "spacebar" => 0x20,
        s if s.len() == 1 && s.as_bytes()[0].is_ascii_alphanumeric() => {
            s.as_bytes()[0].to_ascii_uppercase() as u32
        }
        s => {
            0x6f + s
                .strip_prefix('f')?
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=12).contains(n))?
        }
    })
}
fn chord(value: &str) -> Option<Chord> {
    let mut result = Chord {
        key: 0,
        modifiers: 0,
    };
    // A malformed alternative must not silently become a different Win32 chord.
    for part in value.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => result.modifiers |= 2,
            "alt" => result.modifiers |= 1,
            "shift" => result.modifiers |= 4,
            "meta" | "cmd" | "command" | "win" => result.modifiers |= 8,
            key if result.key == 0 => result.key = lowercase_key(key)?,
            _ => return None,
        }
    }
    (result.key != 0).then_some(result)
}
fn reserved(c: Chord) -> bool {
    match c.modifiers {
        0 => [
            8,
            9,
            0x1b,
            0x2e,
            0x71,
            0x73,
            0x75,
            b'Q' as u32,
            b'W' as u32,
            b'E' as u32,
            b'R' as u32,
        ]
        .contains(&c.key),
        2 => {
            b"CVOHSZY E123"
                .iter()
                .filter(|b| **b != b' ')
                .any(|b| *b as u32 == c.key)
                || c.key == 0x0d
        }
        4 => c.key == b'1' as u32,
        6 => c.key == b'4' as u32 || c.key == b'P' as u32,
        3 => c.key == 0x20,
        _ => false,
    }
}
impl Snapshot {
    /// For ordered conflicts, accept only the first entry; explicitly reject beyond the native 128-entry budget, without silent truncation or taking core keys.
    pub fn shortcuts(&self) -> Result<Vec<Shortcut>, Error> {
        let mut items: Vec<_> = self.0.contributions["shortcuts"].iter().collect();
        items.sort_by(|a, b| {
            a.order
                .unwrap_or(0)
                .cmp(&b.order.unwrap_or(0))
                .then(a.id.cmp(&b.id))
        });
        let mut occupied = BTreeSet::new();
        let mut bindings = Vec::new();
        for item in items {
            if self.owner(&item.scope_id).is_none() {
                continue;
            }
            let Some(command) = item.command_id.as_deref() else {
                continue;
            };
            if !self.0.contributions["commands"].iter().any(|c| {
                c.command_id.as_deref().unwrap_or(&c.id) == command
                    && c.plugin_id == item.plugin_id
                    && c.scope_id == item.scope_id
            }) {
                continue;
            }
            let Some(p) = item.payload() else {
                continue;
            };
            let Some(keys) = p.get("keys").and_then(Value::as_str) else {
                continue;
            };
            if !text(keys, 2048) || keys.split('/').count() > 16 {
                return Err(Error::Budget);
            }
            for chord in keys.split('/').filter_map(chord) {
                if reserved(chord) || !occupied.insert(chord) {
                    continue;
                }
                if bindings.len() == 128 {
                    return Err(Error::Budget);
                }
                bindings.push(Shortcut {
                    id: item.id.clone(),
                    command: command.into(),
                    scope: item.scope_id.clone(),
                    chord,
                    global: p.get("global") == Some(&Value::Bool(true)),
                });
            }
        }
        Ok(bindings)
    }
    pub fn shortcut_available(&self, id: &str, context: &Context<'_>) -> bool {
        self.0.contributions["shortcuts"]
            .iter()
            .find(|s| s.id == id)
            .is_some_and(|s| {
                self.owner(&s.scope_id).is_some()
                    && s.predicate.as_ref().unwrap().evaluate(context)
                    && s.command_id
                        .as_deref()
                        .and_then(|id| self.command(id, context))
                        .is_some_and(|c| c.scope() == s.scope_id && c.permissions().is_some())
            })
    }
}
#[cfg(test)]
#[path = "loom_extension_shortcuts_tests.rs"]
mod tests;
