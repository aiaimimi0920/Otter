//! Application UI threads exclusively own extension hotkeys; catalog revocation stops routing immediately, registration failures never claim global availability, and no hooks/polling are added.
use super::State;
use crate::{
    loom_extension_snapshot::{Chord, Shortcut, Snapshot},
    workspace_id::UnitId,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
pub(super) type Target = Rc<RefCell<Option<UnitId>>>;
struct Registered {
    definition: Option<Shortcut>,
    id: i32,
    registered: bool,
}
pub(super) struct Owner {
    pub(super) target: Target,
    pub(super) catalog: Option<Arc<Snapshot>>,
    bindings: Vec<Registered>,
    next: i32,
}
impl Owner {
    pub(super) fn new(target: Target) -> Self {
        Self {
            target,
            catalog: None,
            bindings: Vec::new(),
            next: 0x5900,
        }
    }
    pub(super) unsafe fn sync(
        &mut self,
        catalog: Option<Arc<Snapshot>>,
    ) -> std::result::Result<(), String> {
        self.sync_with(catalog, |id, chord, enabled| {
            if enabled {
                RegisterHotKey(
                    None,
                    id,
                    HOT_KEY_MODIFIERS(chord.modifiers) | MOD_NOREPEAT,
                    chord.key,
                )
            } else {
                UnregisterHotKey(None, id)
            }
        })
    }
    fn sync_with(
        &mut self,
        catalog: Option<Arc<Snapshot>>,
        mut change: impl FnMut(i32, Chord, bool) -> Result<()>,
    ) -> std::result::Result<(), String> {
        if match (&self.catalog, &catalog) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        } {
            return Ok(());
        }
        let definitions = catalog.as_ref().map(|s| s.shortcuts()).transpose();
        // Revoke old routes even if new-catalog parsing fails; UI projection errors must not keep old permissions effective.
        self.catalog = catalog;
        let mut errors = Vec::new();
        let definitions = match definitions {
            Ok(v) => v.unwrap_or_default(),
            Err(e) => {
                errors.push(format!("extension shortcut declarations: {e:?}"));
                Vec::new()
            }
        };
        let mut old: Vec<_> = self.bindings.drain(..).map(Some).collect();
        let mut next = Vec::new();
        let mut additions = Vec::new();
        for definition in definitions {
            if let Some(r) = old.iter_mut().find(|r| {
                r.as_ref()
                    .is_some_and(|r| r.definition.as_ref() == Some(&definition))
            }) {
                next.push(r.take().unwrap());
            } else {
                additions.push(definition);
            }
        }
        for mut r in old.into_iter().flatten() {
            if r.registered {
                let chord = r.definition.as_ref().map_or(
                    Chord {
                        key: 0,
                        modifiers: 0,
                    },
                    |d| d.chord,
                );
                if let Err(e) = change(r.id, chord, false) {
                    errors.push(format!("unregister extension shortcut {}: {e}", r.id));
                    r.definition = None; // Failed-unregistration records retain cleanup ownership only and must never keep executing.
                    next.push(r);
                }
            }
        }
        for definition in additions {
            if self.next > 0xbfff || next.len() >= 256 {
                errors.push("extension shortcut registration budget exhausted".into());
                break;
            }
            let id = self.next;
            self.next += 1; // Never reuse IDs; queued old WM_HOTKEY messages must not match new commands.
            let registered = definition.global
                && match change(id, definition.chord, true) {
                    Ok(()) => true,
                    Err(e) => {
                        errors.push(format!(
                            "register extension shortcut {}: {e}",
                            definition.id
                        ));
                        false
                    }
                };
            next.push(Registered {
                definition: Some(definition),
                id,
                registered,
            });
        }
        self.bindings = next;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
    pub(super) fn local(&self, key: u32, modifiers: impl FnOnce() -> u32) -> Option<&Shortcut> {
        // Ordinary unbound keys do not query keyboard state; failed-unregistration cleanup records are excluded from candidates.
        let mut candidates = self
            .bindings
            .iter()
            .filter_map(|r| r.definition.as_ref())
            .filter(|d| d.chord.key == key)
            .peekable();
        candidates.peek()?;
        let modifiers = modifiers();
        candidates.find(|d| d.chord.modifiers == modifiers)
    }
    pub(super) fn global(&self, wp: WPARAM, lp: LPARAM) -> Option<&Shortcut> {
        let value = lp.0 as u32;
        self.bindings
            .iter()
            .find(|r| r.registered && r.id as usize == wp.0)
            .and_then(|r| r.definition.as_ref())
            .filter(|d| {
                d.chord
                    == Chord {
                        key: value >> 16,
                        modifiers: value & 0xffff & !MOD_NOREPEAT.0,
                    }
            })
    }
    pub(super) unsafe fn shutdown(&mut self) {
        self.catalog = None;
        for r in self.bindings.drain(..).filter(|r| r.registered) {
            if let Err(e) = UnregisterHotKey(None, r.id) {
                eprintln!("native-extension-shortcut-cleanup: {e}");
            }
        }
        self.target.borrow_mut().take();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            self.shutdown();
        }
    }
}
pub(super) fn observe(state: &State, message: u32, wp: WPARAM) {
    if matches!(
        message,
        WM_MOUSEACTIVATE | WM_LBUTTONDOWN | WM_NCLBUTTONDOWN
    ) || message == WM_ACTIVATE && wp.0 as u16 != WA_INACTIVE as u16
    {
        let mut target = state.extension_target.borrow_mut();
        if target.as_ref() != Some(&state.unit_id) {
            *target = Some(state.unit_id.clone());
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_shortcuts_tests.rs"]
mod tests;
