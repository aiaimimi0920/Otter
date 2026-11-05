//! 普通EXE真实Art/UIA/菜单/Checkpoint到隔离TLS投送，预览不可冒充正式结果。
use super::*;
use serde_json::Value;
use std::time::Duration;
const SOURCE: &str = include_str!("projection_source.js");
fn art(document: &Value) -> &Value {
    document["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["artId"] == package::ART)
        .unwrap()
}
fn formal(fixture: &fixture::Fixture, mode: &str) -> Option<Value> {
    fixture
        .workspace()
        .filter(|document| results::ready(document, mode))
}
fn verify(document: &Value, original: &Value, target: &str, mode: &str, color: [u8; 4]) {
    let units = document["stickers"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    let source = units
        .iter()
        .find(|unit| unit["id"] == "product-source")
        .unwrap();
    assert_eq!(results::decode(&source["src"]), results::decode(original));
    assert_eq!(document["extension"]["productArtPreserved"], true);
    let node = art(document);
    assert_eq!(
        (node["w"].as_f64(), node["h"].as_f64()),
        (
            Some(f64::from(peer::projection::oracle::LOGICAL.0)),
            Some(f64::from(peer::projection::oracle::LOGICAL.1))
        )
    );
    assert_eq!(node["opacityNormal"], 1.0);
    assert_eq!(node["id"], target);
    assert_eq!(node["outputs"]["mode"], mode);
    let encoded = node["outputs"]["output_image"]
        .as_str()
        .unwrap()
        .strip_prefix("data:image/png;base64,")
        .unwrap();
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    peer::projection::verify_png(&bytes, color);
}
fn revision(document: &Value) -> Option<u64> {
    art(document)["projectionSenders"][0]["link"]["revision"].as_u64()
}
fn sender_requests(state: &peer::State) -> Vec<String> {
    // 普通产品仍可轮询空收件箱；停止发送不能借此关闭无关接收能力。
    state
        .requests
        .iter()
        .filter(|path| path.starts_with("/v1/projections/") && !path.ends_with("/inbox"))
        .cloned()
        .collect()
}
#[test]
#[ignore = "requires explicit owned foreground activation; real ordinary EXE and isolated TLS only"]
fn ordinary_product_art_formal_result_continuous_projection_preview_stop_and_restore() {
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = fixture::Fixture::new();
    let original = fixture.workspace().unwrap()["stickers"][0]["src"].clone();
    let mut peer = peer::Peer::with_source(&fixture.root, SOURCE);
    peer.state.lock().unwrap().projection.enabled = true;
    let ui = ui::Ui::new();
    let foreground = foreground::Owner::new();
    let mut first = fixture.launch("projection-online", &peer.origin);
    foreground.allow(&mut first);
    let source_hwnd = workspace(&mut first);
    select(&ui, &mut first, source_hwnd, &peer.origin);
    ui.command(&mut first, source_hwnd, "获取 / 续签授权会话");
    assert!(!ui.status(
        &mut first,
        source_hwnd,
        "最近一次授权成功；使用时仍须校验有效期"
    ));
    let surface = open_button(&ui, &mut first, source_hwnd, "Commit Red");
    ui.invoke(&ui.named(surface, "Commit Red").unwrap());
    let red = ui::until(&mut first, "red formal checkpoint", |_| {
        formal(&fixture, "red")
    });
    let target = art(&red)["id"].as_str().unwrap().to_owned();
    verify(&red, &original, &target, "red", [255, 0, 0, 255]);
    let art_hwnd = ui::until(&mut first, "Art result workspace", |pid| {
        ui::windows(pid, "Otter.Native.Workspace", None, None)
            .into_iter()
            .find(|hwnd| *hwnd != source_hwnd)
    });
    ui.command_in(&mut first, art_hwnd, "投送目标", "刷新投送目标");
    assert!(!ui.status_in(&mut first, art_hwnd, "投送目标", "投送目标：1 项"));
    ui.command_in(
        &mut first,
        art_hwnd,
        "投送目标",
        "Product Receiver · 自动 · 同一 Loom",
    );
    let first_saved = ui::until(&mut first, "first projection checkpoint", |_| {
        formal(&fixture, "red").filter(|document| revision(document) == Some(1))
    });
    let envelope = first_saved["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["id"] == target)
        .unwrap()["projectionSenders"][0]["link"]["envelope"]
        .clone();
    assert!(envelope["projectionId"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(envelope["source"]["unitId"], target);
    assert_eq!(
        peer.state.lock().unwrap().projection.envelope.as_ref(),
        Some(&envelope)
    );
    let outbox = std::fs::read(fixture.root.join("projection-sender.json")).unwrap();
    let reads = peer.state.lock().unwrap().projection.reads;
    ui.invoke(&ui.named(surface, "Preview Blue").unwrap());
    ui::until(
        &mut first,
        "preview remains read-only for two cycles",
        |_| {
            let state = peer.state.lock().unwrap();
            assert_eq!(state.projection.revision, 1);
            (state.projection.reads >= reads + 2).then_some(())
        },
    );
    verify(
        &fixture.workspace().unwrap(),
        &original,
        &target,
        "red",
        [255, 0, 0, 255],
    );
    ui.invoke(&ui.named(surface, "Commit Green").unwrap());
    let green = ui::until(&mut first, "green projection checkpoint", |_| {
        formal(&fixture, "green").filter(|document| revision(document) == Some(2))
    });
    verify(&green, &original, &target, "green", [0, 255, 0, 255]);
    assert_eq!(
        art(&green)["projectionSenders"][0]["link"]["envelope"],
        envelope
    );
    assert_eq!(
        std::fs::read(fixture.root.join("projection-sender.json")).unwrap(),
        outbox
    );
    ui.command_in(
        &mut first,
        art_hwnd,
        "投送目标",
        "停止投送：Product Receiver",
    );
    ui::until(&mut first, "projection stop checkpoint", |_| {
        fixture
            .workspace()
            .filter(|document| art(document)["projectionSenders"] == json!([]))
    });
    assert!(!ui.status_in(&mut first, art_hwnd, "投送目标", "发送关联已停止"));
    assert!(peer.state.lock().unwrap().projection.unlinked);
    let stopped_requests = sender_requests(&peer.state.lock().unwrap());
    ui.invoke(&ui.named(surface, "Commit Blue").unwrap());
    let mut blue_submitted = false;
    let blue = ui::until(
        &mut first,
        "new formal result after stopped projection",
        |_| {
            if !blue_submitted && ui.named(surface, "Commit Blue submitted").is_some() {
                blue_submitted = true;
                eprintln!("art-product-stage: blue-public-sdk-handler-submitted");
            }
            formal(&fixture, "blue")
        },
    );
    verify(&blue, &original, &target, "blue", [0, 0, 255, 255]);
    std::thread::sleep(Duration::from_millis(1600));
    assert_eq!(
        sender_requests(&peer.state.lock().unwrap()),
        stopped_requests
    );
    assert_eq!(
        art(&fixture.workspace().unwrap())["projectionSenders"],
        json!([])
    );
    unsafe { PostMessageW(Some(surface), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap() };
    ui::until(&mut first, "closed Art surface", |pid| {
        ui::windows(pid, "Otter.Native.Surface", None, None)
            .is_empty()
            .then_some(())
    });
    ui.exit(&mut first, source_hwnd);
    peer.stop();
    let mut restored = fixture.launch("projection-restored", &peer.origin);
    foreground.allow(&mut restored);
    let source_hwnd = workspace(&mut restored);
    ui::until(&mut restored, "restored Art workspace", |pid| {
        (ui::windows(pid, "Otter.Native.Workspace", None, None).len() == 2).then_some(())
    });
    let document = fixture.workspace().unwrap();
    verify(&document, &original, &target, "blue", [0, 0, 255, 255]);
    assert_eq!(art(&document)["projectionSenders"], json!([]));
    ui.exit(&mut restored, source_hwnd);
    peer.assert_no_offline_connections();
    let state = peer.state.lock().unwrap();
    assert_eq!(state.projection.frames.len(), 2);
    let mut frames = Vec::new();
    for (index, png) in state.projection.frames.iter().enumerate() {
        let revision = index + 1;
        let name = format!("receiver-revision-{revision}.png");
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(fixture.root.join(&name))
            .unwrap()
            .write_all(png)
            .unwrap();
        frames.push(json!({"file":name,"revision":revision,"digest":package::digest(png),"width":2,"height":2}));
    }
    fixture.record("validated-art-projection.json", &json!({"actualProduct":true,"realMenus":true,"formalArtResults":true,
        "previewNotPublished":true,"continuousSameIdentity":true,"stopPreventsNewFormalSend":true,"checkpointRestart":true,
        "originalUnchanged":true,"normalExit":true,"frames":frames,"requests":state.requests,
        "realLoom":false,"receiverUiPresented":false,"realDeviceProjection":false,"finalPackage":false}));
}
