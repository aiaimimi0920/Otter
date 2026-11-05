//! 两个普通EXE的实际菜单/编辑/接收/重开/停止；由真实双Loom父夹具显式启动。
use super::*;
use serde_json::Value;
#[path = "offline_joint_fixture.rs"]
mod files;
#[path = "offline_joint_oracle.rs"]
mod oracle;
#[path = "../support/product.rs"]
mod pinned;

fn source(document: &Value) -> &Value {
    document["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "product-source")
        .unwrap()
}
fn source_frame(document: &Value) -> native::pin_geometry::PinRect {
    let text = serde_json::to_string(source(document)).unwrap();
    let window: native::session_window::Window<'_> = serde_json::from_str(&text).unwrap();
    let metadata: native::session_coordinates::Metadata =
        serde_json::from_value(document[native::session_coordinates::KEY].clone()).unwrap();
    metadata
        .space()
        .unwrap()
        .physical(window.expanded_rect().unwrap())
        .unwrap()
}
fn source_workspace(side: &files::Side, product: &mut fixture::Product) -> HWND {
    // 保存可能规范化位置；从当前canonical的明确source身份和坐标空间定位，不按枚举顺序猜。
    let frame = source_frame(&side.files.workspace().unwrap());
    ui::until(product, "visible canonical source workspace", |pid| {
        let matches: Vec<_> = ui::windows(pid, "Otter.Native.Workspace", None, None)
            .into_iter()
            .filter(|hwnd| unsafe {
                let mut rect = RECT::default();
                GetWindowRect(*hwnd, &mut rect).is_ok()
                    && (rect.left, rect.top, rect.right, rect.bottom)
                        == (
                            frame.x,
                            frame.y,
                            frame.x + frame.width as i32,
                            frame.y + frame.height as i32,
                        )
            })
            .collect();
        assert!(matches.len() <= 1, "ambiguous canonical source workspace");
        matches.first().copied()
    })
}
fn received(side: &files::Side, revision: u64) -> Option<Value> {
    let document = side.files.workspace()?;
    assert_eq!(document["extension"]["offlineProductPreserved"], true);
    oracle::row(source(&document), 2);
    let units = document["stickers"].as_array()?;
    assert!(units.len() <= 2);
    units
        .iter()
        .find(|v| v["qrProjection"].is_object() && v["qrProjection"]["revision"] == revision)
        .cloned()
}
fn sender_revision(side: &files::Side, revision: u64) -> Option<Value> {
    let doc = side.files.workspace()?;
    assert_eq!(doc["stickers"].as_array().unwrap().len(), 1);
    let binding = &source(&doc)["projectionSenders"][0];
    (binding["link"]["revision"] == revision).then(|| binding.clone())
}
fn verify(row: &Value, side: &files::Side, binding: &Value, revision: u64) {
    assert_eq!(row["qrProjection"]["envelope"], binding["link"]["envelope"]);
    assert_eq!(row["qrProjection"]["digest"], binding["link"]["digest"]);
    assert_eq!(
        row["qrProjection"]["offlineTransport"]["origin"],
        side.origin
    );
    assert_eq!(
        row["qrProjection"]["otterNativeReceive"]["peerId"],
        side.peer
    );
    assert_eq!(row["qrProjection"]["envelope"]["serverOrigin"], side.source);
    oracle::row(row, revision);
}
fn authorize(
    ui: &ui::Ui,
    fg: &foreground::Owner,
    side: &files::Side,
    product: &mut fixture::Product,
) -> HWND {
    fg.allow(product);
    let hwnd = source_workspace(side, product);
    select(ui, product, hwnd, &side.origin);
    ui.command(product, hwnd, "获取 / 续签授权会话");
    assert!(!ui.status(product, hwnd, "最近一次授权成功；使用时仍须校验有效期"));
    hwnd
}
fn phase(root: &std::path::Path, name: &str, binding: &Value, row: &Value) {
    files::publish(
        root,
        &format!("{name}.json"),
        &json!({"binding":binding,"received":row["qrProjection"],"pixelsMatch":true}),
    );
    files::wait(root, &format!("{name}-audited"));
}
fn inbox_ready(ui: &ui::Ui, product: &mut fixture::Product, hwnd: HWND, accept: &str) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let menu = ui.menu(product, hwnd);
        let ready = ui
            .find_menu_row(menu, accept)
            .is_some_and(|row| unsafe { row.CurrentIsEnabled().unwrap().as_bool() });
        ui.dismiss(product, menu);
        if ready {
            return;
        }
        assert!(
            product.0.try_wait().unwrap().is_none(),
            "receiver exited before inbox ready"
        );
        assert!(
            std::time::Instant::now() < end,
            "ordinary inbox menu deadline"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
#[ignore = "explicit real two Looms, pinned ordinary EXE and owned foreground window only"]
fn ordinary_product_offline_two_loom_send_receive_reset_flip_reopen_and_stop() {
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let root = files::root();
    let product_pin =
        pinned::Product::resolve(std::path::Path::new(env!("CARGO_BIN_EXE_otter-native"))).unwrap();
    assert_eq!(
        product_pin.path(),
        std::path::PathBuf::from(std::env::var_os("OTTER_ART_PRODUCT_EXE").unwrap())
    );
    let a = files::Side::load(&root, true);
    let b = files::Side::load(&root, false);
    assert_ne!(a.origin, b.origin);
    assert_eq!(a.source, a.origin);
    assert_eq!(b.source, a.origin);
    let ui = ui::Ui::new();
    files::publish(&root, "started.json", &json!({"ordinaryProducts":true}));
    // 父端先把本测试进程登记进独占Job，再放行产品spawn；失败也不能留下孤儿产品。
    files::wait(&root, "parent-ready");
    let fg = foreground::Owner::new();
    let mut sender = a.launch("sender-online");
    let sh = authorize(&ui, &fg, &a, &mut sender);
    files::publish(
        &root,
        "sender-authorized.json",
        &json!({"deviceSession":true}),
    );
    let mut receiver = b.launch("receiver-online");
    let rh = authorize(&ui, &fg, &b, &mut receiver);
    ui.command(&mut receiver, rh, "接收：手动");
    ui::until(&mut receiver, "saved ordinary receive policy", |_| {
        let bytes = std::fs::read(b.files.root.join("projection-receive-policy.json")).ok()?;
        let value: Value = serde_json::from_slice(&bytes).ok()?;
        (value["policy"] == "confirm").then_some(())
    });
    files::publish(
        &root,
        "authorized.json",
        &json!({"twoDeviceSessions":true,"realPolicyMenu":true}),
    );
    fg.allow(&mut sender);
    ui.command_in(&mut sender, sh, "投送目标", "刷新投送目标");
    assert!(!ui.status_in(&mut sender, sh, "投送目标", "投送目标：1 项"));
    ui.command_in(
        &mut sender,
        sh,
        "投送目标",
        "Ordinary offline receiver B · 手动 · 离线互信 · Explicit offline peer",
    );
    let first = ui::until(&mut sender, "ordinary first sender checkpoint", |_| {
        sender_revision(&a, 1)
    });
    assert_eq!(first["link"]["otterNativeSend"]["target"], a.target);
    assert_eq!(first["target"]["deviceId"], b.target);
    assert_eq!(first["target"]["peerId"], a.peer);
    assert_eq!(first["link"]["offlineTransport"]["origin"], a.origin);
    fg.allow(&mut receiver);
    ui.command(&mut receiver, rh, "刷新投送收件箱");
    let id = first["link"]["envelope"]["projectionId"].as_str().unwrap();
    let accept = format!(
        "离线 · 接受投送：Ordinary offline sender A · {}",
        &id[11..19]
    );
    inbox_ready(&ui, &mut receiver, rh, &accept);
    ui.command(&mut receiver, rh, &accept);
    let dialog = ui::until(&mut receiver, "owned projection confirmation", |pid| {
        ui::windows(pid, "#32770", Some("接受投送"), Some(rh))
            .first()
            .copied()
    });
    unsafe {
        ui.invoke(
            &ui.client
                .ElementFromHandle(GetDlgItem(Some(dialog), IDYES.0).unwrap())
                .unwrap(),
        );
    }
    let row = ui::until(&mut receiver, "ordinary saved received PNG", |_| {
        received(&b, 1)
    });
    verify(&row, &b, &first, 1);
    let unit = row["id"].clone();
    let received_hwnd = ui::until(
        &mut receiver,
        "ordinary visible received workspace",
        |pid| {
            ui::windows(pid, "Otter.Native.Workspace", None, None)
                .into_iter()
                .find(|v| *v != rh)
        },
    );
    assert!(unsafe { IsWindowVisible(received_hwnd) }.as_bool());
    assert_eq!(
        unsafe { GetForegroundWindow() },
        rh,
        "new delivery must not activate"
    );
    assert!(!ui.status(&mut receiver, rh, "投送已呈现"));
    phase(&root, "first", &first, &row);
    fg.allow(&mut sender);
    ui.command(&mut sender, sh, "重置裁剪");
    let reset = ui::until(&mut sender, "ordinary crop reset sender checkpoint", |_| {
        sender_revision(&a, 2)
    });
    let row = ui::until(&mut receiver, "ordinary reset received PNG", |_| {
        received(&b, 2)
    });
    verify(&row, &b, &reset, 2);
    assert_eq!(row["id"], unit);
    phase(&root, "reset", &reset, &row);
    fg.allow(&mut sender);
    ui.command(&mut sender, sh, "水平翻转");
    let flipped = ui::until(&mut sender, "ordinary flip sender checkpoint", |_| {
        sender_revision(&a, 3)
    });
    assert_eq!(flipped["link"]["envelope"], first["link"]["envelope"]);
    let row = ui::until(&mut receiver, "ordinary flipped received PNG", |_| {
        received(&b, 3)
    });
    verify(&row, &b, &flipped, 3);
    assert_eq!(row["id"], unit);
    phase(&root, "flipped", &flipped, &row);
    fg.allow(&mut receiver);
    ui.exit(&mut receiver, rh);
    let mut receiver = b.launch("receiver-reopened");
    let rh = authorize(&ui, &fg, &b, &mut receiver);
    let row = ui::until(&mut receiver, "ordinary reopened received image", |_| {
        received(&b, 3)
    });
    verify(&row, &b, &flipped, 3);
    assert_eq!(row["id"], unit);
    files::publish(
        &root,
        "reopened.json",
        &json!({"sameUnit":true,"binding":row["qrProjection"],"pixelsMatch":true}),
    );
    fg.allow(&mut sender);
    ui.command_in(
        &mut sender,
        sh,
        "投送目标",
        "停止投送：Ordinary offline receiver B",
    );
    ui::until(&mut sender, "ordinary stopped canonical", |_| {
        a.files
            .workspace()
            .filter(|v| source(v)["projectionSenders"] == json!([]))
    });
    let last = ui::until(&mut receiver, "ordinary receiver terminal saved", |_| {
        received(&b, 3).filter(|v| v["qrProjection"]["stopped"] == true)
    });
    verify(&last, &b, &flipped, 3);
    assert_eq!(last["qrProjection"]["stopReason"], "projection_unlinked");
    files::publish(
        &root,
        "stopped.json",
        &json!({"lastImagePreserved":true,"received":last["qrProjection"],"senderRemoved":true}),
    );
    files::wait(&root, "stop-audited");
    fg.allow(&mut receiver);
    ui.exit(&mut receiver, rh);
    let mut receiver = b.launch("receiver-terminal-reopened");
    fg.allow(&mut receiver);
    let rh = source_workspace(&b, &mut receiver);
    let restored = ui::until(&mut receiver, "ordinary reopened terminal", |_| {
        received(&b, 3)
    });
    assert_eq!(restored["qrProjection"], last["qrProjection"]);
    oracle::row(&restored, 3);
    ui.exit(&mut receiver, rh);
    fg.allow(&mut sender);
    ui.exit(&mut sender, sh);
    files::publish(
        &root,
        "finished.json",
        &json!({"ordinaryProducts":true,"realMenus":true,"cropReset":true,
        "flipAppliedOnce":true,"receiverReopened":true,"terminalReopened":true,"normalExit":true,
        "receiverVisible":true,"deliveryDidNotActivate":true,"crossMachine":false,"finalPackage":false}),
    );
}

#[test]
fn source_window_identity_tracks_saved_geometry_and_coordinate_space() {
    let mut doc = json!({"otterNativeCoordinateSpace":{"version":1,"origin":[-100,20],"scale":1.5},
        "stickers":[{"id":"unrelated","x":40,"y":40,"w":1,"h":1},
            {"id":"product-source","x":40,"y":40,"w":160,"h":80,
                "imageEditState":{"sourceSize":{"w":32,"h":16},"cropRect":{"x":4,"y":2,"w":16,"h":8}}}]});
    let initial = source_frame(&doc);
    assert_eq!(
        (initial.x, initial.y, initial.width, initial.height),
        (-40, 80, 240, 120)
    );
    doc["stickers"][1]["x"] = json!(80);
    doc["stickers"][1]["y"] = json!(60);
    let saved = source_frame(&doc);
    assert_eq!(
        (saved.x, saved.y, saved.width, saved.height),
        (20, 110, 240, 120)
    );
}
