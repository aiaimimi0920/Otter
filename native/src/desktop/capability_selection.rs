//! 原生目录选择使用不可变快照，菜单索引不跨目录版本解释；选择本身不执行能力。
use super::{capability_job::Loaded, State};
use std::rc::Rc;
use windows::{
    core::{w, Result, HSTRING},
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
};
const FIRST: usize = 0x6000;
pub(super) unsafe fn append_menu(parent: HMENU, state: &State) -> Result<Option<Snapshot>> {
    let snapshot = Snapshot::capture(state);
    super::capability_catalog::append(parent, state)?;
    super::capability_parameters::append(parent, state)?;
    super::execution_start::append(parent, state)?;
    super::execution_settings::append(parent, state)?;
    super::sticker_propagation::append(parent, state)?;
    if let Some(snapshot) = &snapshot {
        snapshot.append(parent, state)?;
    }
    Ok(snapshot)
}
pub(super) struct Selection {
    pub execution: crate::graph_execution_config::Config,
    pub identity: crate::workspace_id::UnitId,
    pub catalog: Rc<Loaded>,
    pub parameters: crate::loom_parameter_state::Parameters,
}
pub(super) struct Snapshot {
    catalog: Rc<Loaded>,
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
impl Snapshot {
    pub(super) fn capture(state: &State) -> Option<Self> {
        Some(Self {
            catalog: state.catalog.borrow().loaded.clone()?,
        })
    }
    pub(super) unsafe fn append(&self, parent: HMENU, state: &State) -> Result<()> {
        let menu = Menu(CreatePopupMenu()?);
        let selected = state.selected_art.borrow();
        for index in 0..self.catalog.handshake.arts().len() {
            let art = self
                .catalog
                .handshake
                .capability(index)
                .expect("validated art identity");
            let id = art.id;
            let label = art.label.replace('&', "&&");
            let enabled = art.enabled;
            let checked = selected.as_ref().is_some_and(|selected| {
                Rc::ptr_eq(&selected.catalog, &self.catalog) && selected.parameters.art_id() == id
            });
            AppendMenuW(
                menu.0,
                MF_STRING
                    | if enabled { MF_ENABLED } else { MF_GRAYED }
                    | if checked { MF_CHECKED } else { MF_UNCHECKED },
                FIRST + index,
                &HSTRING::from(label),
            )?;
        }
        if self.catalog.handshake.arts().is_empty() {
            AppendMenuW(menu.0, MF_STRING | MF_GRAYED, 0, w!("没有可用节点能力"))?;
        }
        AppendMenuW(parent, MF_POPUP, menu.0 .0 as usize, w!("选择节点能力"))?;
        // 父菜单接管子菜单，异常路径由局部 guard 回收。
        std::mem::forget(menu);
        Ok(())
    }
    pub(super) fn handles(&self, command: usize) -> bool {
        command
            .checked_sub(FIRST)
            .is_some_and(|index| index < self.catalog.handshake.arts().len())
    }
    pub(super) unsafe fn select(&self, hwnd: HWND, state: &State, command: usize) -> Result<bool> {
        if !IsWindow(Some(hwnd)).as_bool() || state.group_locked.get() {
            return Ok(false);
        }
        let Some(index) = command.checked_sub(FIRST) else {
            return Ok(false);
        };
        let Some(art) = self.catalog.handshake.capability(index) else {
            return Ok(false);
        };
        if !art.enabled
            || !state
                .catalog
                .borrow()
                .loaded
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &self.catalog))
        {
            return Ok(false);
        }
        let id = art.id;
        if state
            .selected_art
            .borrow()
            .as_ref()
            .is_some_and(|selected| {
                Rc::ptr_eq(&selected.catalog, &self.catalog) && selected.parameters.art_id() == id
            })
        {
            return Ok(false);
        }
        // 先构造完整候选，失败不能取消旧执行或留下半初始化的选择。
        let parameters = crate::loom_parameter_state::Parameters::new(&art).map_err(|_| {
            windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "能力参数初始化失败",
            )
        })?;
        let identity = super::document_state::new_unit_id()?;
        super::execution_cancel::request(hwnd, state)?;
        *state.selected_art.borrow_mut() = Some(Selection {
            execution: art.execution_config(None),
            identity,
            catalog: self.catalog.clone(),
            parameters,
        });
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    fn catalog() -> Rc<Loaded> {
        let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
        let mut value = crate::loom_handshake::tests::fixture();
        value["capabilities"]["artDefinitions"][0]["parameters"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"amount","type":"number","default":"42"}));
        Rc::new(Loaded {
            identity,
            handshake: crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .unwrap(),
        })
    }
    #[test]
    fn selection_rejects_replaced_catalog_and_captures_only_previous_execution() {
        let state = State::new(None, false).unwrap();
        state.catalog.borrow_mut().loaded = Some(catalog());
        let stale = Snapshot::capture(&state).unwrap();
        state.catalog.borrow_mut().loaded = Some(catalog());
        let current = Snapshot::capture(&state).unwrap();
        let request = crate::graph_requests::Request {
            id: crate::workspace_id::UnitId::parse("selection-old-request").unwrap(),
            generation: 1,
        };
        state.execution_results.borrow_mut().begin(request.clone());
        unsafe {
            // 内置 STATIC 类的隐藏窗口；不注册热键、不捕获桌面、不连接真实服务。
            let window = Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("Otter selection test"),
                    WINDOW_STYLE::default(),
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            assert!(!stale.select(window.0, &state, FIRST).unwrap());
            assert!(state.selected_art.borrow().is_none());
            assert!(state.execution_cancel.borrow().is_none());
            state.group_locked.set(true);
            assert!(!current.select(window.0, &state, FIRST).unwrap());
            state.group_locked.set(false);
            assert!(current.select(window.0, &state, FIRST).unwrap());
            let cancellation = state.execution_cancel.borrow_mut().take().unwrap();
            assert_eq!(cancellation.execution, Some(request));
            assert!(cancellation.preparation.is_none());
            assert_eq!(
                state
                    .selected_art
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .parameters
                    .art_id(),
                "test-art"
            );
            assert!(!current.select(window.0, &state, FIRST).unwrap());
            assert!(state.execution_cancel.borrow().is_none());
            assert!(state.pending_execution.borrow().is_none());
            {
                let selected = state.selected_art.borrow();
                let params = &selected.as_ref().unwrap().parameters;
                assert_eq!(params.get("amount"), Some(Some(&serde_json::json!(42.0))));
                assert!(params.get("secret").is_none());
                assert_eq!(params.revision(), 0);
            }
            let parent = Menu(CreatePopupMenu().unwrap());
            current.append(parent.0, &state).unwrap();
            assert_ne!(
                GetMenuState(GetSubMenu(parent.0, 0), FIRST as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
            let previous = state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .catalog
                .clone();
            let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
            let mut invalid = crate::loom_handshake::tests::fixture();
            invalid["capabilities"]["artDefinitions"][0]["parameters"] =
                serde_json::json!([{"id":"bad\nname","default":0}]);
            let handshake = crate::loom_handshake::response(&serde_json::to_vec(&invalid).unwrap())
                .unwrap()
                .unwrap();
            state.catalog.borrow_mut().loaded = Some(Rc::new(Loaded {
                identity,
                handshake,
            }));
            let invalid = Snapshot::capture(&state).unwrap();
            assert!(invalid.select(window.0, &state, FIRST).is_err());
            assert!(Rc::ptr_eq(
                &previous,
                &state.selected_art.borrow().as_ref().unwrap().catalog
            ));
            assert!(state.execution_cancel.borrow().is_none());
            assert!(state.execution_results.borrow().active().is_some());
        }
    }
    #[test]
    fn menu_keeps_disabled_capabilities_unselectable_and_parent_owns_children() {
        let state = State::new(None, false).unwrap();
        let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
        let mut value = crate::loom_handshake::tests::fixture();
        value["capabilities"]["artDefinitions"][0]["enabled"] = false.into();
        let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
            .unwrap()
            .unwrap();
        state.catalog.borrow_mut().loaded = Some(Rc::new(Loaded {
            identity,
            handshake,
        }));
        let snapshot = Snapshot::capture(&state).unwrap();
        unsafe {
            let parent = Menu(CreatePopupMenu().unwrap());
            snapshot.append(parent.0, &state).unwrap();
            let child = GetSubMenu(parent.0, 0);
            assert_eq!(GetMenuItemCount(Some(child)), 1);
            assert_ne!(
                GetMenuState(child, FIRST as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
            assert!(!snapshot.handles(FIRST + 1));
            assert!(!snapshot.select(HWND::default(), &state, FIRST).unwrap());
        }
        assert!(state.selected_art.borrow().is_none());
    }
}
