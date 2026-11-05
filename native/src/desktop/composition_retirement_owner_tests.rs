//! 隔离进程验证真实 GPU owner 的保留；注入失败凭证，不伪称真实驱动故障。
use super::*;
use crate::desktop::{Apartment, Window};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};

#[test]
#[ignore = "intentionally retains a real GPU graph; run alone in an isolated test process"]
fn failed_retirement_keeps_complete_d3d11_owner_and_budget() {
    let _apartment = Apartment::new().unwrap();
    let before = lifetime::Lease::used_for_test();
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("retirement ownership"),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let mut composition = Composition::new(
            window.0,
            D2D_SIZE_U {
                width: 32,
                height: 32,
            },
        )
        .unwrap();
        let weak = std::rc::Rc::downgrade(composition.state._shared_device.as_ref().unwrap());
        assert_eq!(lifetime::Lease::used_for_test(), before + 1);
        composition.state.retirement = Some(retirement::Retirement::failed_for_test());
        assert!(composition.ensure_attached().is_err());
        drop(composition);
        assert!(
            weak.upgrade().is_some(),
            "unconfirmed GPU owner released its shared device lease"
        );
        assert_eq!(
            lifetime::Lease::used_for_test(),
            before + 1,
            "unconfirmed graph returned its budget"
        );
        let hwnd = window.0;
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        // 有意保留到本隔离进程退出；不得与最后租约释放测试放在同一个进程。
        println!("native-d3d11-retirement-retention: real-owner=true synthetic-failure=true budget-retained=true hwnd-destroyed=true");
    }
}
