//! 冻结帧选择的真实隐藏 HWND 消息验证；不读取桌面、不注入系统输入。
use super::*;
fn image(width: u32, height: u32) -> Image {
    Image {
        width,
        height,
        pixels: vec![255; (width * height * 4) as usize].into(),
    }
}
#[test]
fn hidden_native_region_selects_synthetic_pixels_and_cancels_without_document() {
    for cancel in [false, true] {
        let image = Image {
            width: 4,
            height: 4,
            pixels: (0..64).collect::<Vec<u8>>().into(),
        };
        let result = select_positioned_with(
            image,
            POINT { x: -1920, y: -1080 },
            Vec::new(),
            false,
            |hwnd| unsafe {
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(1 | (1 << 16))));
                SendMessageW(hwnd, WM_LBUTTONUP, None, Some(LPARAM(3 | (3 << 16))));
                SendMessageW(
                    hwnd,
                    WM_KEYDOWN,
                    Some(WPARAM(
                        if cancel { VK_ESCAPE.0 } else { VK_RETURN.0 } as usize
                    )),
                    None,
                );
            },
        )
        .unwrap();
        if cancel {
            assert!(result.is_none());
        } else {
            let (image, origin) = result.unwrap();
            assert_eq!((origin.x, origin.y), (-1919, -1079));
            assert_eq!((image.width, image.height), (2, 2));
            assert_eq!(&image.pixels[..8], &[20, 21, 22, 23, 24, 25, 26, 27]);
        }
    }
    for click in [false, true] {
        let windows = vec![
            RECT {
                left: 20,
                top: 20,
                right: 70,
                bottom: 60,
            },
            RECT {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
        ];
        let selected = select_with(
            image(100, 100),
            POINT::default(),
            windows,
            false,
            |hwnd| unsafe {
                let point = Some(LPARAM(40 | (40 << 16)));
                SendMessageW(hwnd, WM_MOUSEMOVE, None, point);
                if click {
                    SendMessageW(hwnd, WM_LBUTTONDOWN, None, point);
                    SendMessageW(hwnd, WM_LBUTTONUP, None, point);
                }
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!((selected.width, selected.height), (50, 40));
    }
}
#[test]
fn live_drag_and_double_click_share_selector_without_cropping_frozen_pixels() {
    let image = image(100, 100);
    let original = image.pixels.clone();
    for whole in [false, true] {
        let rect = RECT {
            left: 20,
            top: 20,
            right: 80,
            bottom: 70,
        };
        let result = select_bounds_with(
            &image,
            POINT {
                x: -30000,
                y: -30000,
            },
            vec![rect],
            false,
            true,
            |hwnd| unsafe {
                let at = |x: isize, y: isize| Some(LPARAM(x | (y << 16)));
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, at(30, 30));
                SendMessageW(
                    hwnd,
                    WM_LBUTTONUP,
                    None,
                    at(if whole { 30 } else { 60 }, if whole { 30 } else { 50 }),
                );
                if whole {
                    SendMessageW(hwnd, WM_LBUTTONDBLCLK, None, at(30, 30));
                }
                // 若正式完成分支失效，取消会使测试失败而不是挂住消息循环。
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.1, whole);
        assert_eq!(
            result.0,
            if whole {
                rect
            } else {
                RECT {
                    left: 30,
                    top: 30,
                    right: 60,
                    bottom: 50,
                }
            }
        );
        assert!(image.pixels.same_storage(&original));
    }
    let result = select_bounds_with(
        &image,
        POINT {
            x: -30000,
            y: -30000,
        },
        vec![],
        false,
        true,
        |hwnd| unsafe {
            SendMessageW(hwnd, WM_LBUTTONDBLCLK, None, Some(LPARAM(10 | (10 << 16))));
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        },
    )
    .unwrap();
    assert!(result.is_none());
}
