//! 窗口级 UIA provider、可撤销 Fragment 树与经权限门禁的操作模式。
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Com::*, UI::Accessibility::*},
};
#[path = "surface_uia_root_abi.rs"]
mod abi;
#[cfg(test)]
#[path = "surface_uia_range_tests.rs"]
pub(super) mod range_tests;

pub(super) struct Root {
    provider: IRawElementProviderSimple,
    alive: Arc<AtomicBool>,
    scene: Arc<Mutex<Option<crate::surface_accessibility::Handle>>>,
}
impl Root {
    pub(super) fn new(hwnd: HWND) -> Result<Self> {
        let mut apartment = APTTYPE::default();
        let mut qualifier = APTTYPEQUALIFIER::default();
        unsafe { CoGetApartmentType(&mut apartment, &mut qualifier)? };
        if apartment != APTTYPE_STA && apartment != APTTYPE_MAINSTA {
            return Err(Error::from_hresult(RPC_E_WRONG_THREAD));
        }
        let alive = Arc::new(AtomicBool::new(true));
        let scene = Arc::new(Mutex::new(None));
        Ok(Self {
            provider: abi::create(hwnd, alive.clone(), scene.clone()),
            alive,
            scene,
        })
    }
    pub(super) fn provider(&self) -> IRawElementProviderSimple {
        self.provider.clone()
    }
    pub(super) fn node(
        &self,
        handle: crate::surface_accessibility::Handle,
    ) -> Result<IRawElementProviderSimple> {
        abi::node(&self.provider, handle)
    }
    pub(super) fn set_scene(
        &self,
        scene: Option<crate::surface_accessibility::Handle>,
    ) -> Result<()> {
        *self.scene.lock().map_err(|_| Error::from_hresult(E_FAIL))? = scene;
        Ok(())
    }
}
pub(super) fn notify_toggle(provider: &IRawElementProviderSimple, before: bool, after: bool) {
    abi::notify_toggle(provider, before, after);
}
pub(super) fn notify_range(provider: &IRawElementProviderSimple, before: f64, after: f64) {
    abi::notify_range(provider, before, after);
}
pub(super) fn notify_range_readonly(
    provider: &IRawElementProviderSimple,
    before: bool,
    after: bool,
) {
    abi::notify_range_readonly(provider, before, after);
}
impl Drop for Root {
    fn drop(&mut self) {
        // 先撤销再释放 COM 引用；外部持有的 provider 不得继续读取已销毁的 HWND。
        self.alive.store(false, Ordering::Release);
    }
}

pub(super) fn destroyed(hwnd: HWND) {
    // 官方 WM_DESTROY 清理入口；不在 SendMessage 同步路径调用 UiaDisconnectProvider。
    unsafe { UiaReturnRawElementProvider(hwnd, WPARAM(0), LPARAM(0), None) };
}

#[cfg(test)]
#[path = "surface_uia_client_tests.rs"]
mod client_tests;
#[cfg(test)]
#[path = "surface_uia_focus_tests.rs"]
mod focus_tests;
#[cfg(test)]
#[path = "surface_uia_fragment_tests.rs"]
mod fragment_tests;
#[cfg(test)]
#[path = "surface_uia_geometry_tests.rs"]
mod geometry_tests;
#[cfg(test)]
#[path = "surface_uia_hwnd_override_tests.rs"]
mod hwnd_override_tests;
#[cfg(test)]
#[path = "surface_uia_invoke_tests.rs"]
mod invoke_tests;
#[cfg(test)]
#[path = "surface_uia_process_tests.rs"]
mod process_tests;
#[cfg(test)]
pub(super) use process_tests::fixture::Fixture as ProcessFixture;
#[cfg(test)]
#[path = "surface_uia_root_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "surface_uia_toggle_tests.rs"]
mod toggle_tests;
