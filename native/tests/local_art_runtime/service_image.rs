//! 同份公开serviceImage示例在普通EXE的菜单、UIA、原签名包/TLS和checkpoint链。
//! 只触及自启PID及独立profile；像素预期复用独立PNG解码常量，不调用生产渲染器。
use super::*;

#[test]
#[ignore = "requires short owned foreground menus and the independently hashed public SDK example"]
fn ordinary_product_service_image_menu_formal_checkpoint_restart_and_close() {
    let source_path =
        std::path::PathBuf::from(std::env::var_os("OTTER_ART_IMAGE_EXAMPLE").unwrap());
    assert!(source_path.is_absolute());
    let source = std::fs::read_to_string(source_path).unwrap();
    assert_eq!(
        package::digest(source.as_bytes()),
        std::env::var("OTTER_ART_IMAGE_SHA256").unwrap()
    );
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = fixture::Fixture::new();
    let original = fixture.workspace().unwrap()["stickers"][0]["src"].clone();
    let mut peer = peer::Peer::with_source(&fixture.root, &source);
    let ui = ui::Ui::new();
    let foreground = foreground::Owner::new();
    let mut first = fixture.launch("online-image", &peer.origin);
    foreground.allow(&mut first);
    let hwnd = workspace(&mut first);
    select(&ui, &mut first, hwnd, &peer.origin);
    ui.command(&mut first, hwnd, "获取 / 续签授权会话");
    assert!(!ui.status(&mut first, hwnd, "最近一次授权成功；使用时仍须校验有效期"));
    let surface = open_button(&ui, &mut first, hwnd, "运行");
    ui.invoke(&ui.named(surface, "运行").unwrap());
    let online = ui::until(&mut first, "service image formal checkpoint", |_| {
        fixture
            .workspace()
            .filter(|v| results::ready(v, "image_output"))
    });
    let target = results::verify(&online, &original, "image_output");
    assert!(peer.state.lock().unwrap().image_output_owner.is_none());
    ui.exit(&mut first, hwnd);
    peer.stop();
    let requests = peer.state.lock().unwrap().requests.clone();
    for (path, count) in [
        ("/v1/art/local/resource", 3),
        ("/v1/art/local/service/submit-image-output", 1),
        ("/v1/art/local/service/cancel", 1),
    ] {
        assert_eq!(
            requests.iter().filter(|p| p.as_str() == path).count(),
            count
        );
    }
    assert!(!requests.iter().any(|p| p.ends_with("/service/submit")));
    let mut restored = fixture.launch("restored-image", &peer.origin);
    foreground.allow(&mut restored);
    let hwnd = workspace(&mut restored);
    ui::until(&mut restored, "restored Art result workspace", |pid| {
        ui::windows(pid, "Otter.Native.Workspace", None, None)
            .into_iter()
            .find(|w| *w != hwnd)
    });
    assert_eq!(
        results::verify(&fixture.workspace().unwrap(), &original, "image_output"),
        target
    );
    ui.exit(&mut restored, hwnd);
    peer.assert_no_offline_connections();
    assert_eq!(peer.state.lock().unwrap().requests, requests);
    fixture.record("validated-image-output.json", &json!({
        "actualProduct":true,"menus":true,"publicExampleSha256":package::digest(source.as_bytes()),
        "serviceImage":true,"formalPixels":true,"originalUnchanged":true,"checkpointRestart":true,
        "restoredWorkspace":true,"normalExit":true,"requests":requests,"target":target,
        "ownedPids":[first.0.id(),restored.0.id()],"restoredNetworkConnections":false,
        "realLoom":false,"physicalInput":false,"visiblePixelOracle":false,"finalPackage":false
    }));
}
