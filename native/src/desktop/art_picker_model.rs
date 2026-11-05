//! 不可变目录/源凭证及有界搜索；WSS metadata 不授予 HTTPS 权限，不拼接服务地址。
use super::*;
use desktop::{configuration_stamp::Stamp, popup_menu::target::MenuTarget};
pub(in crate::desktop) enum Route {
    Backend,
    Local(String),
}
pub(in crate::desktop) struct Request {
    pub catalog: Rc<Loaded>,
    pub index: usize,
    pub route: Route,
    snapshot: Snapshot,
}
pub(in crate::desktop) struct Snapshot {
    pub catalog: Option<Rc<Loaded>>,
    pub origin: Option<String>,
    unit: crate::workspace_id::UnitId,
    owner: HWND,
    marker: isize,
    stamp: Stamp,
    target: MenuTarget,
}
impl Snapshot {
    pub fn capture(hwnd: HWND, state: &State) -> Self {
        let view = state.authorization.borrow();
        Self {
            catalog: state.catalog.borrow().loaded.clone(),
            origin: view.origin.clone(),
            unit: state.unit_id.clone(),
            owner: hwnd,
            marker: state as *const State as isize,
            stamp: Stamp::capture(state),
            target: MenuTarget::capture(&state.canvas.borrow()),
        }
    }
    pub fn current(&self, hwnd: HWND, state: &State) -> bool {
        self.owner == hwnd
            && self.marker == state as *const State as isize
            && unsafe { owns(hwnd, state) }
            && self.unit == state.unit_id
            && self.stamp.matches(state)
            && self.target.matches(&state.canvas.borrow())
            && match (&self.catalog, &state.catalog.borrow().loaded) {
                (Some(old), Some(current)) => Rc::ptr_eq(old, current),
                (None, None) => true,
                _ => false,
            }
    }
    #[cfg(test)]
    pub fn search(&self, query: &str) -> Vec<usize> {
        search(self.catalog.as_deref(), query)
    }
    pub fn route(&self, index: usize, state: &State) -> Option<Route> {
        route(
            self.catalog.as_deref(),
            index,
            &self.origin,
            &state.authorization.borrow(),
        )
    }
    pub fn request(&self, index: usize, state: &State) -> Option<Request> {
        let route = self.route(index, state)?;
        let mut snapshot = Self::capture(self.owner, state);
        if !self.current(self.owner, state) {
            return None;
        }
        snapshot.origin = self.origin.clone();
        Some(Request {
            catalog: self.catalog.clone()?,
            index,
            route,
            snapshot,
        })
    }
}
pub(in crate::desktop) fn search(catalog: Option<&Loaded>, query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    catalog
        .map(|catalog| {
            (0..catalog.handshake.arts().len())
                .filter(|index| {
                    catalog.handshake.capability(*index).is_some_and(|art| {
                        art.id.to_lowercase().contains(&query)
                            || art.label.to_lowercase().contains(&query)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
pub(in crate::desktop) fn route(
    catalog: Option<&Loaded>,
    index: usize,
    origin: &Option<String>,
    view: &desktop::device_authorization_ui::View,
) -> Option<Route> {
    let art = catalog?.handshake.capability(index)?;
    if !art.enabled {
        return None;
    }
    if !art.runtime().surface {
        return Some(Route::Backend);
    }
    if view.closed
        || view.busy
        || view.mount_busy
        || view.request.is_some()
        || &view.origin != origin
    {
        return None;
    }
    // 目录只提供提示；显式 HTTPS 仍必须经过 describe/prepare/审批，不能由 WSS 推导。
    origin.clone().map(Route::Local)
}
impl Request {
    pub fn current(&self, hwnd: HWND, state: &State) -> bool {
        self.snapshot.current(hwnd, state)
            && available(state)
            && match (&self.route, self.snapshot.route(self.index, state)) {
                (Route::Backend, Some(Route::Backend)) => true,
                (Route::Local(origin), Some(Route::Local(current))) => *origin == current,
                _ => false,
            }
    }
    pub unsafe fn queue(self, hwnd: HWND, state: &State) -> Result<()> {
        if !self.current(hwnd, state) || state.art_request.borrow().is_some() {
            return Ok(());
        }
        *state.art_request.borrow_mut() = Some(self);
        if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
            state.art_request.borrow_mut().take();
            return Err(error);
        }
        Ok(())
    }
}
pub(in crate::desktop) fn primary_input(art: &crate::loom_capability::Capability<'_>) -> String {
    art.inputs()
        .find(|port| port.is_image_like() || port.is_primary_image())
        .or_else(|| art.inputs().next())
        .map_or("input_image", |port| port.name)
        .into()
}
