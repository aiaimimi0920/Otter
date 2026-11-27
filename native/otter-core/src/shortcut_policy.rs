//! Built-in keyboard policy. Command identities are independent of the keys assigned to them.
use serde::{Deserialize, Serialize};

pub const MAX_REVISION: u64 = (1 << 53) - 1;
pub const ALT: u8 = 1;
pub const CTRL: u8 = 2;
pub const SHIFT: u8 = 4;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    GamesOnly,
    AllFullscreen,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fullscreen {
    pub enabled: bool,
    pub scope: Scope,
}

impl Default for Fullscreen {
    fn default() -> Self {
        Self {
            enabled: false,
            scope: Scope::GamesOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Chord {
    pub key: u8,
    pub modifiers: u8,
}

impl Chord {
    fn valid(self, global: bool) -> bool {
        let Self { key, modifiers } = self;
        // Only US virtual letters, digits and F1..F11. Never admit Windows-key
        // bindings, F12 (debugger), shell navigation, IME or modal keys.
        if modifiers & !(CTRL | ALT | SHIFT) != 0
            || !(key.is_ascii_uppercase() || key.is_ascii_digit() || (0x70..=0x7a).contains(&key))
            || (key == 0x73 && modifiers & ALT != 0)
        {
            return false;
        }
        if global {
            return modifiers & (CTRL | ALT) != 0 || (0x70..=0x7a).contains(&key);
        }
        // Fixed editing commands, clipboard, colors and style shortcuts remain
        // outside the tool namespace even when the corresponding tool is unbound.
        if modifiers & CTRL != 0
            && matches!(
                key,
                b'A' | b'C' | b'D' | b'E' | b'H' | b'O' | b'S' | b'V' | b'X' | b'Y' | b'Z'
            )
        {
            return false;
        }
        modifiers & ALT == 0
            && key != 0x73
            && (modifiers & CTRL != 0 || !matches!(key, b'C' | b'D' | b'F' | b'O' | b'Q'))
            && !(modifiers == SHIFT && key == b'1')
    }

    pub fn label(self) -> String {
        let mut s = String::new();
        for (bit, label) in [(CTRL, "Ctrl+"), (ALT, "Alt+"), (SHIFT, "Shift+")] {
            if self.modifiers & bit != 0 {
                s.push_str(label);
            }
        }
        if (0x70..=0x7a).contains(&self.key) {
            s.push_str(&format!("F{}", self.key - 0x6f));
        } else {
            s.push(char::from(self.key));
        }
        s
    }
}

pub const GLOBAL: &[(&str, u8)] = &[
    ("capture.region", b'1'),
    ("capture.live", b'2'),
    ("capture.scroll", b'3'),
    ("capture.delay", 0),
    ("pin.restore-input", 0),
    ("shortcuts.toggle", 0),
];
pub const CANVAS: &[(&str, u8)] = &[
    ("tool.pan", b'V'),
    ("tool.select", b'S'),
    ("tool.brush", b'B'),
    ("tool.line", b'L'),
    ("tool.bezier", b'K'),
    ("tool.paintbrush", b'J'),
    ("tool.text", b'X'),
    ("tool.serial", b'N'),
    ("tool.mosaic", b'M'),
    ("tool.blur", b'G'),
    ("tool.eraser", b'W'),
    ("tool.rectangle", b'R'),
    ("tool.ellipse", b'E'),
    ("tool.triangle", b'T'),
    ("tool.polygon", b'P'),
    ("tool.spot.rect", 0),
    ("tool.spot.round", 0),
    ("tool.spot.ellipse", 0),
    ("tool.magnifier", 0),
];
pub fn canvas_command(id: &str) -> Option<usize> {
    Some(match id {
        "tool.spot.rect" => 0x21f1,
        "tool.spot.round" => 0x21f2,
        "tool.spot.ellipse" => 0x21f3,
        "tool.magnifier" => 0x21f4,
        _ => usize::from(
            CANVAS
                .iter()
                .find(|(action, key)| *action == id && *key != 0)?
                .1,
        ),
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub action: String,
    // Explicit null means unbound. Omission must not silently unbind a command.
    #[serde(deserialize_with = "Option::deserialize")]
    pub chord: Option<Chord>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub bindings: Vec<Binding>,
    pub fullscreen: Fullscreen,
}

impl Policy {
    pub fn chord(&self, action: &str) -> Option<Chord> {
        if let Some(binding) = self.bindings.iter().find(|b| b.action == action) {
            return binding.chord;
        }
        GLOBAL
            .iter()
            .map(|pair| (pair, CTRL))
            .chain(CANVAS.iter().map(|pair| (pair, 0)))
            .find(|((id, key), _)| *id == action && *key != 0)
            .map(|((_, key), modifiers)| Chord {
                key: *key,
                modifiers,
            })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.bindings.len() > GLOBAL.len() + CANVAS.len() {
            return Err("shortcut_binding_limit");
        }
        for (index, b) in self.bindings.iter().enumerate() {
            let global = GLOBAL.iter().any(|(id, _)| *id == b.action);
            if !global && !CANVAS.iter().any(|(id, _)| *id == b.action) {
                return Err("shortcut_unknown_action");
            }
            if b.chord.is_some_and(|chord| !chord.valid(global)) {
                return Err("shortcut_reserved_key");
            }
            if self.bindings[..index]
                .iter()
                .any(|previous| previous.action == b.action)
            {
                return Err("shortcut_duplicate_action");
            }
        }
        for domain in [GLOBAL, CANVAS] {
            for (i, (id, _)) in domain.iter().enumerate() {
                if let Some(chord) = self.chord(id) {
                    if domain[..i]
                        .iter()
                        .any(|(prior, _)| self.chord(prior) == Some(chord))
                    {
                        return Err("shortcut_conflict");
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding(id: &str, key: u8, modifiers: u8) -> Binding {
        Binding {
            action: id.into(),
            chord: Some(Chord { key, modifiers }),
        }
    }
    #[test]
    fn defaults_unbound_swap_and_collision() {
        let mut p = Policy::default();
        assert!(p.validate().is_ok());
        assert_eq!(
            p.fullscreen,
            Fullscreen {
                enabled: false,
                scope: Scope::GamesOnly
            }
        );
        p.bindings.push(binding("capture.region", b'2', CTRL));
        assert_eq!(p.validate(), Err("shortcut_conflict"));
        p.bindings.push(binding("capture.live", b'1', CTRL));
        assert!(p.validate().is_ok());
        p.bindings[0].chord = None;
        assert_eq!(p.chord("capture.region"), None);
        assert!(p.validate().is_ok());
    }
    #[test]
    fn invalid_wire_and_reserved_keys_fail_closed() {
        assert!(serde_json::from_str::<Binding>(r#"{"action":"capture.region"}"#).is_err());
        for (id, key, mods) in [
            ("capture.live", b'A', 0),
            ("capture.live", 0x73, ALT),
            ("capture.live", b'A', 8),
            ("tool.blur", b'S', CTRL),
            ("tool.blur", b'C', 0),
            ("tool.unknown", b'A', 0),
        ] {
            let p = Policy {
                bindings: vec![binding(id, key, mods)],
                ..Default::default()
            };
            assert!(p.validate().is_err());
        }
    }
}
