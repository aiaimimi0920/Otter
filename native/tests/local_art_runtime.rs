#![cfg(windows)]
//! 实际普通产品、原菜单/对话框、完整资源Art、服务进度、正式checkpoint及跨进程离线。
#[path = "local_art_runtime/fixture.rs"]
mod fixture;
#[path = "local_art_runtime/foreground.rs"]
mod foreground;
#[path = "local_art_runtime/offline_joint.rs"]
mod offline_joint;
#[path = "local_art_runtime/package.rs"]
mod package;
#[path = "local_art_runtime/peer.rs"]
mod peer;
#[path = "local_art_runtime/projection.rs"]
mod projection;
#[path = "local_art_runtime/results.rs"]
mod results;
#[path = "local_art_runtime/service_image.rs"]
mod service_image;
#[path = "local_art_runtime/ui.rs"]
mod ui;
use otter_native as native;
use serde_json::json;
use windows::Win32::{
    Foundation::*,
    UI::{HiDpi::*, WindowsAndMessaging::*},
};
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
fn workspace(product: &mut fixture::Product) -> HWND {
    let hwnd = ui::until(product, "visible source workspace", |pid| {
        ui::windows(pid, "Otter.Native.Workspace", None, None)
            .into_iter()
            .find(|hwnd| unsafe {
                let mut rect = RECT::default();
                GetWindowRect(*hwnd, &mut rect).is_ok() && rect.left == 40 && rect.top == 40
            })
    });
    // 只给自启测试窗口加唯一caption，供原生控制面识别；不改业务状态或窗口样式。
    unsafe {
        SetWindowTextW(
            hwnd,
            &windows::core::HSTRING::from(format!("Otter Art Product Test {}", product.0.id())),
        )
        .unwrap()
    };
    hwnd
}
fn select(ui: &ui::Ui, product: &mut fixture::Product, hwnd: HWND, origin: &str) {
    ui.command(product, hwnd, "选择 Loom 服务…");
    ui.input(product, hwnd, "选择 Loom 服务", origin);
}
fn open(ui: &ui::Ui, product: &mut fixture::Product, hwnd: HWND) -> HWND {
    open_button(ui, product, hwnd, "Run packaged Art")
}
fn open_button(ui: &ui::Ui, product: &mut fixture::Product, hwnd: HWND, name: &str) -> HWND {
    ui.command(product, hwnd, "打开 Art（原生预览）…");
    ui.input(product, hwnd, "打开 Art（原生预览）", package::ART);
    ui::until(product, "packaged Art first frame", |pid| {
        ui::windows(pid, "Otter.Native.Surface", Some("Otter Surface"), None)
            .into_iter()
            .find(|hwnd| {
                ui.named(*hwnd, name).is_some_and(|button| unsafe {
                    button.CurrentIsEnabled().is_ok_and(|value| value.as_bool())
                })
            })
    })
}
#[test]
fn uia_client_owner_releases_interfaces_before_apartment() {
    for _ in 0..4 {
        drop(ui::Ui::new());
    }
}
#[test]
#[ignore = "requires short owned foreground menus; run explicitly in the MAIN window"]
fn ordinary_product_package_art_menu_service_checkpoint_restart_offline_and_close() {
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = fixture::Fixture::new();
    let original = fixture.workspace().unwrap()["stickers"][0]["src"].clone();
    let mut peer = peer::Peer::new(&fixture.root);
    let ui = ui::Ui::new();
    let foreground = foreground::Owner::new();
    let mut first = fixture.launch("online", &peer.origin);
    foreground.allow(&mut first);
    let hwnd = workspace(&mut first);
    select(&ui, &mut first, hwnd, &peer.origin);
    ui.command(&mut first, hwnd, "获取 / 续签授权会话");
    ui::until(&mut first, "device authorization completed", |_| {
        peer.state
            .lock()
            .unwrap()
            .requests
            .iter()
            .any(|path| path == "/v1/device-sessions")
            .then_some(())
    });
    // 授权worker请求已到达不等于owner已接纳；从菜单状态确认真实产品终态。
    assert!(!ui.status(&mut first, hwnd, "最近一次授权成功；使用时仍须校验有效期"));
    let surface = open(&ui, &mut first, hwnd);
    ui.invoke(&ui.named(surface, "Run packaged Art").unwrap());
    ui::until(&mut first, "real service submit", |_| {
        peer.state
            .lock()
            .unwrap()
            .requests
            .iter()
            .any(|path| path.ends_with("/service/submit"))
            .then_some(())
    });
    assert!(!ui.status(&mut first, hwnd, "Art 服务执行中 37%"));
    peer.state.lock().unwrap().release = true;
    let online = ui::until(&mut first, "formal online checkpoint", |_| {
        fixture.workspace().filter(|v| results::ready(v, "online"))
    });
    let target = results::verify(&online, &original, "online");
    ui.exit(&mut first, hwnd);
    peer.stop();
    let requests = peer.state.lock().unwrap().requests.clone();
    assert_eq!(
        requests
            .iter()
            .filter(|p| p.as_str() == "/v1/art/local/resource")
            .count(),
        3
    );
    assert_eq!(
        requests
            .iter()
            .filter(|p| p.ends_with("/service/submit"))
            .count(),
        1
    );
    let mut second = fixture.launch("offline", &peer.origin);
    foreground.allow(&mut second);
    let hwnd = workspace(&mut second);
    let restored = fixture.workspace().unwrap();
    assert_eq!(results::verify(&restored, &original, "online"), target);
    select(&ui, &mut second, hwnd, &peer.origin);
    // 选择已有Art窗口，原OpenArt复用同节点；不新造第二个Art或设备会话。
    let art = ui::until(&mut second, "restored Art workspace", |pid| {
        ui::windows(pid, "Otter.Native.Workspace", None, None)
            .into_iter()
            .find(|w| *w != hwnd)
    });
    let surface = open(&ui, &mut second, art);
    ui.invoke(&ui.named(surface, "Run packaged Art").unwrap());
    let offline = ui::until(&mut second, "formal offline checkpoint", |_| {
        fixture.workspace().filter(|v| results::ready(v, "offline"))
    });
    assert_eq!(results::verify(&offline, &original, "offline"), target);
    unsafe { PostMessageW(Some(surface), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap() };
    ui::until(&mut second, "Art close reclaims window", |pid| {
        ui::windows(pid, "Otter.Native.Surface", None, None)
            .is_empty()
            .then_some(())
    });
    ui.exit(&mut second, hwnd);
    peer.assert_no_offline_connections();
    assert_eq!(
        peer.state.lock().unwrap().requests,
        requests,
        "offline cannot make network requests"
    );
    fixture.record("validated.json",&json!({"actualProduct":true,"menus":true,"packageResources":3,"serviceProgress":37,
        "formalPixels":true,"checkpointRestart":true,"offlineDeviceSession":false,"target":target,"requests":requests,
        "ownedPids":[first.0.id(),second.0.id()],"normalExit":true}));
}
