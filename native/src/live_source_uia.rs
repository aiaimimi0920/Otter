//! 显式共享来源的只读UIA树；仅受限child内MTA使用，COM接口不跨进程/线程。
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
mod sample;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sample {
    pub id: String,
    pub locator: Locator,
    pub value: std::result::Result<serde_json::Value, String>,
}
pub(crate) fn root_error(reason: &str) -> Sample {
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
                // 密码控件/跨进程子树不读名称、值或后代。失败也不推测为非密码。
                if element.CurrentProcessId()? != target.process_id() as i32
                    || element.CurrentIsPassword()?.as_bool()
                {
                    continue;
                }
                let label = sample::label(&element)?;
                let sampled = sample::read(&element, ancestors.clone(), bounds, is_root)?;
                // Provider属性不是原子快照；至少在返回值/进入后代前再检查隐私与同PID边界。
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
        // windows-core 0.60把成功的null接口转换成Error::empty(S_OK)。
        // 仅树walker的空端点适用；真正E_POINTER/其他失败不能被吞成完整扫描。
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
