//! Read-only UIA tree for explicitly shared sources; MTA use only inside constrained children, with no COM interfaces crossing processes/threads.
use crate::live_relay_observation::Locator;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use windows::{
    core::*,
    Win32::{Foundation::*, System::Com::*, UI::Accessibility::*},
};
#[path = "live_source_uia_process.rs"]
pub mod process;
#[path = "live_source_uia_sample.rs"]
pub mod sample;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub id: String,
    pub locator: Locator,
    pub value: std::result::Result<serde_json::Value, String>,
}
pub fn root_error(reason: &str) -> Sample {
    Sample {
        id: "uia:source".into(),
        locator: Locator {
            automation_id: None,
            name: None,
            control_type: "Window".into(),
            ancestor_path: vec![],
            runtime_id: None,
        },
        value: Err(reason.into()),
    }
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
struct Scanner {
    client: IUIAutomation,
    _apartment: Apartment,
}
impl Scanner {
    fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let apartment = Apartment;
            let automation: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
            automation.SetConnectionTimeout(500)?;
            automation.SetTransactionTimeout(500)?;
            Ok(Self {
                client: automation.cast()?,
                _apartment: apartment,
            })
        }
    }
    fn scan(&self, target: &crate::live_window_target::Target) -> Result<Vec<Sample>> {
        target.validate()?;
        crate::live_window_target::input_security::preflight(target)
            .map_err(|_| Error::from(E_ACCESSDENIED))?;
        unsafe {
            let root = self.client.ElementFromHandle(target.hwnd())?;
            if root.CurrentProcessId()? != target.process_id() as i32 {
                return Err(Error::from(E_ACCESSDENIED));
            }
            let bounds = root.CurrentBoundingRectangle()?;
            let walker = self.client.ControlViewWalker()?;
            let mut samples = BTreeMap::<String, Sample>::new();
            let mut stack = vec![(root, Vec::<String>::new(), true)];
            let mut visited = 0;
            while let Some((element, ancestors, is_root)) = stack.pop() {
                visited += 1;
                if visited > 256 || ancestors.len() > 16 {
                    return Err(Error::from(E_BOUNDS));
                }
                // Do not read names, values, or descendants of password controls/cross-process subtrees. Failures must not imply nonpassword status.
                if element.CurrentProcessId()? != target.process_id() as i32
                    || element.CurrentIsPassword()?.as_bool()
                {
                    continue;
                }
                let identity = sample::Identity::read(&element)?;
                let label = identity.label();
                let sampled = sample::read(&element, identity, ancestors.clone(), bounds, is_root)?;
                // Provider properties are not atomic snapshots; at least recheck privacy and same-PID boundaries before returning values/entering descendants.
                if element.CurrentProcessId()? != target.process_id() as i32
                    || element.CurrentIsPassword()?.as_bool()
                {
                    continue;
                }
                if let Some(value) = sampled {
                    match samples.entry(value.id.clone()) {
                        std::collections::btree_map::Entry::Vacant(slot) => {
                            slot.insert(value);
                        }
                        std::collections::btree_map::Entry::Occupied(mut slot) => {
                            slot.get_mut().value = Err("uia_locator_ambiguous".into());
                        }
                    }
                }
                let mut path = ancestors;
                path.push(label);
                let mut children = Vec::new();
                let mut next = optional(walker.GetFirstChildElement(&element))?;
                while let Some(child) = next {
                    if visited + stack.len() + children.len() >= 256 {
                        return Err(Error::from(E_BOUNDS));
                    }
                    next = optional(walker.GetNextSiblingElement(&child))?;
                    children.push((child, path.clone(), false));
                }
                stack.extend(children.into_iter().rev());
            }
            target.validate()?;
            Ok(samples.into_values().collect())
        }
    }
}
fn optional<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        // windows-core 0.60 converts successful null interfaces into Error::empty(S_OK).
        // Applies only to empty tree-walker endpoints; real E_POINTER/other failures must not be swallowed as complete scans.
        Err(e) if e.code() == S_OK => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_uia_walker_null_success_ends_tree_but_api_failures_propagate() {
        assert_eq!(optional(Ok(7u8)).unwrap(), Some(7));
        assert_eq!(optional::<u8>(Err(Error::empty())).unwrap(), None);
        for code in [E_POINTER, E_FAIL, E_ACCESSDENIED, E_BOUNDS] {
            assert_eq!(
                optional::<u8>(Err(Error::from(code))).unwrap_err().code(),
                code
            );
        }
    }
}
