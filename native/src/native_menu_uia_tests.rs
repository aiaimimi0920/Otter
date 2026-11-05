//! 真实 COM ABI 与节点撤销；跨线程系统发现另由客户端回归覆盖。
#[path = "native_menu_uia_focus_tests.rs"]
mod focus;
#[path = "native_menu_uia_status_tests.rs"]
mod status;
use super::*;
use crate::native_menu::{accessibility::Owner, panel::Row};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};
struct Fixture {
    hwnd: HWND,
    owner: Owner,
}
impl Fixture {
    fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("UIA menu test"),
                WS_POPUP,
                30,
                30,
                200,
                56,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        };
        Self {
            hwnd,
            owner: Owner::new(&[row(0), row(1)]),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.owner.revoke();
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            CoUninitialize();
        }
    }
}
fn row(key: usize) -> Row {
    Row {
        key,
        thumbnail: None,
        label: format!("&命令 {key}\tCtrl+O"),
        enabled: true,
        checked: false,
        submenu: false,
        separator: false,
        focused: false,
        radio: false,
        default: false,
    }
}
#[test]
fn com_tree_names_geometry_identity_and_invoke_follow_snapshot_lifetime() {
    let fixture = Fixture::new();
    let root = create(fixture.hwnd, fixture.owner.view(), 28).unwrap();
    unsafe {
        assert_eq!(
            root.GetPropertyValue(UIA_NamePropertyId)
                .unwrap()
                .to_string(),
            "Otter 菜单"
        );
        let mut unsupported = ptr::dangling_mut();
        assert_eq!(
            (root.vtable().GetPatternProvider)(
                root.as_raw(),
                UIA_InvokePatternId,
                &mut unsupported
            ),
            S_OK
        );
        assert!(unsupported.is_null());
        let fragment: IRawElementProviderFragment = root.cast().unwrap();
        let first = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        let simple: IRawElementProviderSimple = first.cast().unwrap();
        assert_eq!(
            simple
                .GetPropertyValue(UIA_NamePropertyId)
                .unwrap()
                .to_string(),
            "命令 0"
        );
        let rect = first.BoundingRectangle().unwrap();
        assert_eq!(
            (rect.left, rect.top, rect.width, rect.height),
            (30.0, 30.0, 200.0, 28.0)
        );
        let back: IRawElementProviderSimple = first
            .Navigate(NavigateDirection_Parent)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(back.as_raw(), root.as_raw());
        let invoke: IInvokeProvider = simple
            .GetPatternProvider(UIA_InvokePatternId)
            .unwrap()
            .cast()
            .unwrap();
        first.SetFocus().unwrap();
        invoke.Invoke().unwrap();
        assert_eq!(fixture.owner.take(), Some((0, Action::Focus)));
        assert_eq!(fixture.owner.take(), Some((0, Action::Invoke)));
        fixture.owner.replace(&[row(2)]);
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        assert!(first.BoundingRectangle().is_err());
        fixture.owner.revoke();
        assert!(root.GetPropertyValue(UIA_NamePropertyId).is_err());
    }
}

#[test]
fn submenu_pattern_reports_committed_state_and_rejects_stale_or_disabled_requests() {
    let fixture = Fixture::new();
    let mut submenu = row(0);
    submenu.submenu = true;
    fixture.owner.replace(&[submenu.clone()]);
    let root = create(fixture.hwnd, fixture.owner.view(), 28).unwrap();
    unsafe {
        let fragment: IRawElementProviderFragment = root.cast().unwrap();
        let simple: IRawElementProviderSimple = fragment
            .Navigate(NavigateDirection_FirstChild)
            .unwrap()
            .cast()
            .unwrap();
        assert!(simple.cast::<IInvokeProvider>().is_err());
        let expand: IExpandCollapseProvider = simple
            .GetPatternProvider(UIA_ExpandCollapsePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            expand.ExpandCollapseState().unwrap(),
            ExpandCollapseState_Collapsed
        );
        expand.Expand().unwrap();
        assert_eq!(
            expand.ExpandCollapseState().unwrap(),
            ExpandCollapseState_Collapsed
        );
        assert_eq!(fixture.owner.take(), Some((0, Action::Expand)));
        fixture.owner.set_expanded(Some(0));
        assert_eq!(
            expand.ExpandCollapseState().unwrap(),
            ExpandCollapseState_Expanded
        );
        expand.Collapse().unwrap();
        assert_eq!(
            expand.ExpandCollapseState().unwrap(),
            ExpandCollapseState_Expanded
        );
        assert_eq!(fixture.owner.take(), Some((0, Action::Collapse)));
        fixture.owner.set_expanded(None);
        assert_eq!(
            expand.ExpandCollapseState().unwrap(),
            ExpandCollapseState_Collapsed
        );
        submenu.enabled = false;
        fixture.owner.replace(&[submenu]);
        assert_eq!(
            expand.Expand().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let disabled: IRawElementProviderSimple = fragment
            .Navigate(NavigateDirection_FirstChild)
            .unwrap()
            .cast()
            .unwrap();
        let disabled: IExpandCollapseProvider = disabled
            .GetPatternProvider(UIA_ExpandCollapsePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            disabled.Expand().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        assert!(fixture.owner.take().is_none());
    }
}

#[test]
fn root_scroll_pattern_validates_axes_percentages_and_only_queues_bounded_offsets() {
    let fixture = Fixture::new();
    fixture
        .owner
        .set_scroll(crate::native_menu::accessibility::Scroll {
            first: 0,
            visible: 2,
            total: 40,
        });
    let root = create(fixture.hwnd, fixture.owner.view(), 28).unwrap();
    unsafe {
        let scroll: IScrollProvider = root
            .GetPatternProvider(UIA_ScrollPatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert!(!scroll.HorizontallyScrollable().unwrap().as_bool());
        assert!(scroll.VerticallyScrollable().unwrap().as_bool());
        assert_eq!(
            scroll.HorizontalScrollPercent().unwrap(),
            UIA_ScrollPatternNoScroll
        );
        assert_eq!(scroll.VerticalViewSize().unwrap(), 5.0);
        assert!(scroll.SetScrollPercent(-1.0, f64::NAN).is_err());
        assert!(scroll.SetScrollPercent(-1.0, 101.0).is_err());
        assert!(scroll.SetScrollPercent(0.0, 10.0).is_err());
        assert!(scroll
            .Scroll(ScrollAmount(99), ScrollAmount_NoAmount)
            .is_err());
        assert!(fixture.owner.take().is_none());
        scroll.SetScrollPercent(-1.0, 100.0).unwrap();
        assert_eq!(scroll.VerticalScrollPercent().unwrap(), 0.0);
        assert_eq!(
            fixture.owner.take(),
            Some((usize::MAX, Action::ScrollTo(38)))
        );
        scroll
            .Scroll(ScrollAmount_NoAmount, ScrollAmount_LargeIncrement)
            .unwrap();
        assert_eq!(
            fixture.owner.take(),
            Some((usize::MAX, Action::ScrollTo(2)))
        );
        fixture
            .owner
            .set_scroll(crate::native_menu::accessibility::Scroll {
                first: 0,
                visible: 2,
                total: 2,
            });
        assert!(!scroll.VerticallyScrollable().unwrap().as_bool());
        assert_eq!(
            scroll.VerticalScrollPercent().unwrap(),
            UIA_ScrollPatternNoScroll
        );
        assert!(scroll.SetScrollPercent(-1.0, 100.0).is_err());
        fixture.owner.revoke();
        assert_eq!(
            scroll.VerticalViewSize().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
