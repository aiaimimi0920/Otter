use super::*;
use crate::{
    desktop::{Apartment, Window},
    live_frames::Source,
};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};

unsafe fn window() -> Window {
    Window(
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP,
            w!("STATIC"),
            w!(""),
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
    )
}
unsafe fn publish(source: &Source<Texture>, device: &ID3D11Device, color: [u8; 4]) {
    let frame = super::super::tests::frame(device, color);
    source
        .publish(Arc::try_unwrap(frame).ok().unwrap().resource)
        .unwrap();
}
unsafe fn device() -> ID3D11Device {
    let device = create_device(D3D_DRIVER_TYPE_WARP).unwrap();
    let protection: ID3D11Multithread = device.GetImmediateContext().unwrap().cast().unwrap();
    let _ = protection.SetMultithreadProtected(true);
    device
}
#[test]
fn snapshot_uses_latest_pending_without_changing_presented_and_retains_owner_after_close() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let device = device();
        let source = Source::new();
        let mut host = Host::bind(window.0, source.subscribe().unwrap()).unwrap();
        assert!(host.snapshot_frame().unwrap().is_none());
        publish(&source, &device, [0, 0, 255, 255]);
        host.paint(D2D_SIZE_U {
            width: 32,
            height: 32,
        })
        .unwrap();
        assert_eq!(host.snapshot_frame().unwrap().unwrap().sequence, 1);
        publish(&source, &device, [0, 255, 0, 255]);
        let latest = host.snapshot_frame().unwrap().unwrap();
        assert_eq!(latest.sequence, 2);
        assert_eq!(host.presented_viewport([32, 32]).unwrap().0, 1);
        host.suspend();
        assert!(Arc::ptr_eq(
            &latest,
            &host.snapshot_frame().unwrap().unwrap()
        ));
        source.close();
        assert!(host.snapshot_frame().unwrap().is_none());
        let image =
            crate::live_texture::readback::read(latest, &std::sync::atomic::AtomicBool::new(false))
                .unwrap();
        assert_eq!(&*image.pixels, &[0, 255, 0, 255].repeat(4));
        assert!(host.snapshot_frame().unwrap().is_none());
    }
}
#[test]
fn messages_coalesce_until_paint_and_close_releases_frame_and_subscription() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let device = device();
        let source = Source::new();
        let mut host = Host::bind(window.0, source.subscribe().unwrap()).unwrap();
        let size = D2D_SIZE_U {
            width: 32,
            height: 32,
        };
        assert!(!host.paint(size).unwrap());
        assert!(host.presented_viewport([32, 32]).is_none());
        for _ in 0..3 {
            publish(&source, &device, [0, 0, 255, 255]);
        }
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(window.0),
            notification::MESSAGE,
            notification::MESSAGE,
            PM_REMOVE
        )
        .as_bool());
        assert_eq!(
            host.receive(message.message, message.wParam, message.lParam)
                .unwrap(),
            Update::Frame
        );
        assert_eq!(host.pending.as_ref().unwrap().sequence, 3);
        assert!(host.surface.is_none());
        assert!(!host.paint(D2D_SIZE_U::default()).unwrap());
        assert!(host.surface.is_none());
        assert!(host.paint(size).unwrap());
        assert_eq!(host.presented_viewport([32, 32]).unwrap().0, 3);
        assert!(host.presented_viewport([64, 32]).is_none());
        let frame = Arc::downgrade(&host.surface.as_ref().unwrap().frame);
        assert!(host.pending.is_none());
        // 不派发新消息，已有 paint 仍能读取最新帧，补偿被遗漏的通知。
        publish(&source, &device, [0, 255, 0, 255]);
        assert_eq!(host.refresh().unwrap(), Update::Frame);
        // 接收到 pending 帧不等于呈现成功，不能提前切换输入帧身份。
        assert_eq!(host.presented_viewport([32, 32]).unwrap().0, 3);
        assert!(host.paint(size).unwrap());
        assert_eq!(host.presented_viewport([32, 32]).unwrap().0, 4);
        assert!(frame.upgrade().is_none());
        let last = Arc::downgrade(&host.surface.as_ref().unwrap().frame);
        source.close();
        assert_eq!(host.refresh().unwrap(), Update::Closed);
        assert!(host.presented_viewport([32, 32]).is_none());
        assert!(last.upgrade().is_none());
        assert!(!source.has_consumers());
        assert!(!host.paint(size).unwrap());
        assert_eq!(
            host.receive(message.message, message.wParam, message.lParam)
                .unwrap(),
            Update::Ignored
        );
    }
}
#[test]
fn suspension_releases_surface_preserves_latest_and_resumes_without_new_frame() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let device = device();
        let source = Source::new();
        let mut host = Host::bind(window.0, source.subscribe().unwrap()).unwrap();
        let size = D2D_SIZE_U {
            width: 32,
            height: 32,
        };
        publish(&source, &device, [0, 0, 255, 255]);
        assert!(host.paint(size).unwrap());
        let old = Arc::downgrade(&host.surface.as_ref().unwrap().frame);
        host.suspend();
        assert!(host.presented_viewport([32, 32]).is_none());
        assert!(host.surface.is_none());
        assert!(source.has_consumers());
        assert!(Arc::ptr_eq(
            host.pending.as_ref().unwrap(),
            &old.upgrade().unwrap()
        ));
        host.suspend();
        assert!(host.paint(size).unwrap());
        assert!(Arc::ptr_eq(
            &host.surface.as_ref().unwrap().frame,
            &old.upgrade().unwrap()
        ));
        // 新帧优先于已呈现的旧帧，暂停时不得把旧帧重新覆盖回 pending。
        publish(&source, &device, [0, 255, 0, 255]);
        assert_eq!(host.refresh().unwrap(), Update::Frame);
        let latest = Arc::downgrade(host.pending.as_ref().unwrap());
        assert!(!host.paint(D2D_SIZE_U::default()).unwrap());
        assert!(host.surface.is_none());
        assert!(old.upgrade().is_none());
        assert!(host.paint(size).unwrap());
        assert!(Arc::ptr_eq(
            &host.surface.as_ref().unwrap().frame,
            &latest.upgrade().unwrap()
        ));
        host.suspend();
        source.close();
        assert_eq!(host.refresh().unwrap(), Update::Closed);
        assert!(host.pending.is_none() && host.surface.is_none());
        assert!(latest.upgrade().is_none());
    }
}
#[test]
fn opacity_survives_surface_recreation_resize_and_frames_without_baking_pixels() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let device = device();
        let source = Source::new();
        let mut host = Host::bind(window.0, source.subscribe().unwrap()).unwrap();
        host.set_opacity(0.35).unwrap();
        publish(&source, &device, [0, 0, 255, 255]);
        for width in [32, 64, 32] {
            assert!(host.paint(D2D_SIZE_U { width, height: 32 }).unwrap());
            assert_eq!(
                host.surface
                    .as_ref()
                    .unwrap()
                    .composition
                    .opacity_for_test(),
                0.35
            );
            for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
                assert!(host.set_opacity(value).is_err());
                assert_eq!(host.opacity, 0.35);
                assert_eq!(
                    host.surface
                        .as_ref()
                        .unwrap()
                        .composition
                        .opacity_for_test(),
                    0.35
                );
            }
            let image = crate::live_texture::readback::read(
                host.snapshot_frame().unwrap().unwrap(),
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(&*image.pixels, &[0, 0, 255, 255].repeat(4));
            host.suspend();
        }
        publish(&source, &device, [0, 255, 0, 255]);
        assert!(host
            .paint(D2D_SIZE_U {
                width: 32,
                height: 32
            })
            .unwrap());
        assert_eq!(
            host.surface
                .as_ref()
                .unwrap()
                .composition
                .opacity_for_test(),
            0.35
        );
    }
}
#[test]
fn render_error_terminates_subscription_without_retry_and_drop_releases_demand() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let device = device();
        let source = Source::new();
        let mut host = Host::bind(window.0, source.subscribe().unwrap()).unwrap();
        publish(&source, &device, [255, 0, 0, 255]);
        assert!(host
            .paint(D2D_SIZE_U {
                width: 8193,
                height: 1
            })
            .is_err());
        assert!(!source.has_consumers());
        assert!(host.surface.is_none() && host.pending.is_none());
        assert!(host.presented_viewport([32, 32]).is_none());
        assert!(!host
            .paint(D2D_SIZE_U {
                width: 32,
                height: 32
            })
            .unwrap());
        let host = Host::bind(window.0, source.subscribe().unwrap()).unwrap();
        assert!(source.has_consumers());
        drop(host);
        assert!(!source.has_consumers());
    }
}
