//! 创建期部分关联失败的隔离资源寿命门；真实图加合成错误，不模拟驱动故障。
use super::*;
use crate::desktop::{Apartment, Window};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};

#[test]
#[ignore = "intentionally retains a real GPU graph; run alone in an isolated test process"]
fn failed_creation_retains_complete_d3d12_owner_and_original_error() {
    let _apartment = Apartment::new().unwrap();
    let before = lifetime::Lease::used_for_test();
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("creation ownership"),
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
        let state = State::new(
            window.0,
            D2D_SIZE_U {
                width: 32,
                height: 32,
            },
        )
        .unwrap();
        let weak = Rc::downgrade(&state.device);
        let owner = CompositionD3d12(ManuallyDrop::new(state));
        let result = owner.attach_with(|state| {
            // 真实修改合成图后注入错误；未确认退休必须保住 context/buffers/queue/预算。
            state.visual.SetContent(&state.chain)?;
            state.target.SetRoot(&state.visual)?;
            state.retirement = Some(retirement::Retirement::failed_for_test());
            Err(Error::new(E_INVALIDARG, "creation attachment fixture"))
        });
        let failure = result.err().expect("attachment failure must propagate");
        assert!(!failure.fallback_allowed());
        let error = failure.into_error();
        assert_eq!(error.code(), E_INVALIDARG);
        assert!(error.to_string().contains("creation attachment fixture"));
        assert!(
            weak.upgrade().is_some(),
            "partial owner released shared device"
        );
        assert_eq!(lifetime::Lease::used_for_test(), before + 1);
        let hwnd = window.0;
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        println!("native-d3d12-creation-retention: real-owner=true synthetic-failure=true original-error=true budget-retained=true hwnd-destroyed=true");
    }
}
