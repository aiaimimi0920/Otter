//! 隐藏自有窗口的真实设备身份与像素隔离；不捕获桌面、注册输入或显示窗口。
use super::super::{Apartment, Window};
use super::*;
use windows::{
    core::{w, IUnknown},
    Win32::UI::WindowsAndMessaging::*,
};

struct Surface {
    composition: Composition,
    _window: Window,
}
impl Surface {
    unsafe fn new(device: Option<&ID3D11Device>) -> Self {
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("device sharing fixture"),
                WS_POPUP,
                0,
                0,
                64,
                64,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let size = D2D_SIZE_U {
            width: 64,
            height: 64,
        };
        let composition = match device {
            Some(device) => Composition::with_device(window.0, size, device),
            None => Composition::new(window.0, size),
        }
        .unwrap();
        Self {
            composition,
            _window: window,
        }
    }
}
unsafe fn identity(surface: &Composition) -> IUnknown {
    let device: ID3D11Device = surface.state.chain.GetDevice().unwrap();
    device.cast().unwrap()
}
unsafe fn draw(surface: &Composition, color: D2D1_COLOR_F) -> [u8; 4] {
    let renderer = surface.renderer();
    renderer.BeginDraw();
    renderer.Clear(Some(&color));
    renderer.EndDraw(None, None).unwrap();
    super::tests::samples(surface).1
}

#[test]
fn default_pin_surfaces_share_device_but_keep_window_resources_independent() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let first = Surface::new(None);
        let second = Surface::new(None);
        assert_eq!(
            identity(&first.composition),
            identity(&second.composition),
            "simultaneous default pin surfaces allocated separate D3D11 devices"
        );
        assert_ne!(
            first.composition.state.context.as_raw(),
            second.composition.state.context.as_raw()
        );
        assert_ne!(
            first.composition.state.chain.as_raw(),
            second.composition.state.chain.as_raw()
        );
        assert_ne!(
            first.composition.state.visual.as_raw(),
            second.composition.state.visual.as_raw()
        );
        assert_ne!(
            first.composition.state.target.as_raw(),
            second.composition.state.target.as_raw()
        );
        assert_ne!(
            first.composition.state.device.as_raw(),
            second.composition.state.device.as_raw()
        );
    }
}

#[test]
fn pin_surfaces_preserve_alpha_resize_opacity_and_survivor_pixels() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let mut first = Surface::new(None);
        let second = Surface::new(None);
        let red = draw(
            &first.composition,
            D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
        );
        assert!(red[2].abs_diff(128) <= 1 && red[3].abs_diff(128) <= 1);
        assert_eq!(&red[..2], &[0, 0]);
        let blue = draw(
            &second.composition,
            D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        );
        assert_eq!(blue, [255, 0, 0, 255]);
        first.composition.set_opacity(0.35).unwrap();
        first
            .composition
            .resize(D2D_SIZE_U {
                width: 32,
                height: 32,
            })
            .unwrap();
        let green = draw(
            &first.composition,
            D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 0.25,
            },
        );
        assert!(green[1].abs_diff(64) <= 1 && green[3].abs_diff(64) <= 1);
        assert_eq!((green[0], green[2]), (0, 0));
        assert_eq!(super::tests::samples(&second.composition).1, blue);
        assert_eq!(first.composition.state.opacity, 0.35);
        assert_eq!(second.composition.state.opacity, 1.0);
        drop(first);
        assert_eq!(
            draw(
                &second.composition,
                D2D1_COLOR_F {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0
                }
            ),
            [255, 255, 0, 255]
        );
    }
}

#[test]
fn explicit_capture_device_never_joins_default_pin_device_pool() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let device = create_device(D3D_DRIVER_TYPE_WARP).unwrap();
        let explicit = Surface::new(Some(&device));
        let default = Surface::new(None);
        assert_eq!(
            identity(&explicit.composition),
            device.cast::<IUnknown>().unwrap()
        );
        assert_ne!(
            identity(&default.composition),
            identity(&explicit.composition)
        );
    }
}

#[test]
fn ten_default_pins_release_the_last_device_lease_and_reopen_fresh() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let mut surfaces: Vec<_> = (0..10).map(|_| Surface::new(None)).collect();
        let first = identity(&surfaces[0].composition);
        assert!(surfaces
            .iter()
            .all(|surface| identity(&surface.composition) == first));
        drop(first);
        let lease = surfaces[0]
            .composition
            .state
            ._shared_device
            .as_ref()
            .unwrap();
        let weak = std::rc::Rc::downgrade(lease);
        let survivor = surfaces.pop().unwrap();
        drop(surfaces);
        assert!(weak.upgrade().is_some());
        drop(survivor);
        assert!(
            weak.upgrade().is_none(),
            "last pin left a strong device owner cached"
        );
        let reopened = Surface::new(None);
        let next = reopened.composition.state._shared_device.as_ref().unwrap();
        assert_ne!(weak.as_ptr(), std::rc::Rc::as_ptr(next));
    }
}
