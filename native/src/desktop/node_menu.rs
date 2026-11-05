//! 节点菜单的打开时快照集合；统一分发能力选择、参数和连线，索引不跨版本解释。
use super::State;
#[path = "art_picker.rs"]
pub(super) mod art_picker;
use windows::{
    core::Result,
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU},
};
pub(super) struct Snapshot {
    creation: Option<std::rc::Rc<super::link_catalog::Catalog>>,
    capability: Option<super::capability_selection::Snapshot>,
    parameters: Option<super::capability_parameter_menu::Snapshot>,
    links: Option<std::rc::Rc<super::link_menu::Snapshot>>,
}
impl Snapshot {
    pub unsafe fn append(menu: HMENU, state: &State) -> Result<Self> {
        super::application::chrome::ports::append(menu, state)?;
        Ok(Self {
            creation: super::link_create::append(menu, state)?,
            capability: super::capability_selection::append_menu(menu, state)?,
            parameters: super::capability_parameter_menu::append(menu, state)?,
            links: super::link_menu::append(menu, state)?,
        })
    }
    pub unsafe fn apply(self, hwnd: HWND, state: &State, command: usize) -> Result<bool> {
        if command == super::application::chrome::ports::COMMAND {
            super::application::chrome::ports::request(&state.chrome, hwnd);
            return Ok(true);
        }
        if super::link_create::apply(self.creation, hwnd, state, command)? {
            return Ok(true);
        }
        if super::capability_parameter_menu::apply(self.parameters, hwnd, state, command)?
            || super::link_menu::apply(self.links, hwnd, state, command)?
        {
            return Ok(true);
        }
        if let Some(snapshot) = self.capability.filter(|snapshot| snapshot.handles(command)) {
            snapshot.select(hwnd, state, command)?;
            return Ok(true);
        }
        Ok(false)
    }
}
