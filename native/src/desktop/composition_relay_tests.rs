//! 隐藏 HWND 的真实 D2D 上传/交换链读回；不启动在线网络或可见窗口。
use super::*;
use crate::{
    desktop::{Apartment, Window},
    live_relay_frame::{Codec, ColorSpace},
    live_relay_viewer_identity::Snapshot,
    live_relay_viewer_session::ViewerSession,
    pixels::Pixels,
};
use std::sync::Arc;
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};

fn window() -> Window {
    Window(unsafe {
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP,
            w!("STATIC"),
            w!(""),
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
        .unwrap()
    })
}
fn frame(id: u64, width: u32, pixels: Pixels) -> PixelFrame {
    PixelFrame::new(
        Metadata {
            epoch: 1,
            frame_id: id,
            capture_timestamp_ms: 0,
            encode_timestamp_ms: 0,
            width,
            height: 1,
            dropped_frames: 0,
            keyframe: true,
            color_space: ColorSpace::Srgb,
            codec: Codec::RawBgra,
        },
        pixels,
    )
    .unwrap()
}
pub(super) unsafe fn sample(composition: &Composition, x: usize, y: usize) -> [u8; 4] {
    let device: ID3D11Device = composition.state.chain.GetDevice().unwrap();
    let context = device.GetImmediateContext().unwrap();
    let source: ID3D11Texture2D = composition.state.chain.GetBuffer(0).unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    source.GetDesc(&mut desc);
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .unwrap();
    let staging = staging.unwrap();
    context.CopyResource(&staging, &source);
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context
        .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        .unwrap();
    let mut result = [0; 4];
    std::ptr::copy_nonoverlapping(
        (mapped.pData as *const u8).add(y * mapped.RowPitch as usize + x * 4),
        result.as_mut_ptr(),
        4,
    );
    context.Unmap(&staging, 0);
    result
}
#[test]
fn remote_bitmap_upload_reuses_size_and_preserves_premultiplied_pixels() {
    let _apartment = Apartment::new().unwrap();
    let window = window();
    let size = D2D_SIZE_U {
        width: 64,
        height: 64,
    };
    let mut surface = Surface::new(
        window.0,
        size,
        &frame(1, 2, vec![0, 0, 128, 128, 0, 0, 128, 128].into()),
    )
    .unwrap();
    surface.draw(size).unwrap();
    assert_eq!(unsafe { sample(&surface.composition, 0, 0) }, [0, 0, 0, 0]);
    assert_eq!(
        unsafe { sample(&surface.composition, 32, 32) },
        [0, 0, 128, 128]
    );
    let bitmap = surface.bitmap.clone();
    surface
        .set_frame(&frame(2, 2, vec![0, 255, 0, 255, 0, 255, 0, 255].into()))
        .unwrap();
    assert_eq!(bitmap.as_raw(), surface.bitmap.as_raw());
    surface.draw(size).unwrap();
    assert_eq!(
        unsafe { sample(&surface.composition, 32, 32) },
        [0, 255, 0, 255]
    );
    surface
        .set_frame(&frame(3, 1, vec![255, 0, 0, 255].into()))
        .unwrap();
    assert_ne!(bitmap.as_raw(), surface.bitmap.as_raw());
    surface
        .draw(D2D_SIZE_U {
            width: 32,
            height: 32,
        })
        .unwrap();
    assert_eq!(
        unsafe { sample(&surface.composition, 16, 16) },
        [255, 0, 0, 255]
    );
}
#[test]
fn host_releases_uploaded_cpu_frame_and_clears_gpu_resources_on_revocation() {
    let _apartment = Apartment::new().unwrap();
    let window = window();
    let snapshot = Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false}"#, "live:test").unwrap();
    let media =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    media.activate(&snapshot).unwrap();
    let permit = media.admit(1).unwrap();
    let mut host = Host::bind(window.0, media.clone(), permit.clone()).unwrap();
    let business = Arc::new(
        crate::live_relay_viewer_business::Business::from_snapshot(
            br#"{"session":{"observationCapabilities":[]},"observations":[]}"#,
        )
        .unwrap(),
    );
    let business_weak = Arc::downgrade(&business);
    media.publish_business(&permit, business).unwrap();
    assert_eq!(host.refresh(), Update::Idle);
    assert!(host.business().is_some());
    let pixels = Pixels::from(vec![255; 4]);
    let weak = pixels.downgrade();
    media.commit(&permit, frame(1, 1, pixels), false).unwrap();
    assert_eq!(host.refresh(), Update::Frame);
    assert!(weak.upgrade().is_some());
    let size = D2D_SIZE_U {
        width: 64,
        height: 64,
    };
    assert!(host.paint(size).unwrap());
    assert!(weak.upgrade().is_none());
    let bitmap = host.surface_for_test().unwrap().bitmap.as_raw();
    assert!(host.paint(size).unwrap());
    assert_eq!(bitmap, host.surface_for_test().unwrap().bitmap.as_raw());
    media.close();
    assert!(host.business().is_none());
    assert_eq!(host.refresh(), Update::Closed);
    assert!(business_weak.upgrade().is_none());
    assert!(host.surface_for_test().is_none());
    assert!(host.change_handle().is_none());
    assert!(!host.paint(size).unwrap());
    assert!(Host::bind(window.0, media, permit).is_err());
}
