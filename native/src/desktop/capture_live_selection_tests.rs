use super::*;
use windows::core::w;
fn candidate() -> (crate::desktop::Window, Candidate) {
    unsafe {
        let window = crate::desktop::Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Live crop fixture"),
                WS_POPUP,
                -30000,
                -30000,
                200,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let target = Target::bind(window.0).unwrap();
        let bounds = target.frame_bounds().unwrap();
        (window, Candidate { target, bounds })
    }
}
#[test]
fn frozen_identity_resolves_crop_or_whole_and_never_retargets_changed_window() {
    for class in [
        "Progman",
        "WorkerW",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
        "ToolTips_Class32",
    ] {
        assert!(ignored_class(class));
    }
    assert!(!ignored_class("Chrome_WidgetWin_1"));
    for whole in [false, true] {
        let (_window, candidate) = candidate();
        let target = candidate.target.clone();
        let frame = candidate.bounds;
        let screen = RECT {
            left: frame.left + 20,
            top: frame.top + 10,
            right: frame.left + 70,
            bottom: frame.top + 40,
        };
        match resolve(vec![candidate], screen, whole).unwrap() {
            Request::Window(actual) if whole => assert!(actual.same_source(&target)),
            Request::WindowRegion(actual, crop) if !whole => {
                assert!(actual.same_source(&target));
                assert_eq!(
                    crop.pixels(),
                    crate::capture_readback::Region {
                        left: 20,
                        top: 10,
                        width: 50,
                        height: 30
                    }
                );
            }
            _ => panic!("wrong Live source kind"),
        }
    }
    for close in [false, true] {
        let (window, candidate) = candidate();
        let screen = candidate.bounds;
        let (_behind, behind) = self::candidate();
        unsafe {
            if close {
                DestroyWindow(window.0).unwrap();
            } else {
                SetWindowPos(
                    window.0,
                    None,
                    -29900,
                    -29900,
                    200,
                    100,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
                .unwrap();
            }
        }
        assert!(resolve(vec![candidate, behind], screen, false).is_err());
    }
    assert!(resolve(
        vec![],
        RECT {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10
        },
        true
    )
    .is_err());
    assert!(resolve(vec![], RECT::default(), false).is_err());
    assert!(matches!(
        resolve(
            vec![],
            RECT {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1
            },
            false
        )
        .unwrap(),
        Request::Region(_)
    ));
}
