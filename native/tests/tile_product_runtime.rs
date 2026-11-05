#![cfg(windows)]
//! 普通固定SHA产品经真实CLI/物理输出呈现；只采自启PID四个已确认无遮挡的测试点。
use serde_json::{json, Value};
use std::{
    fs,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::*,
    UI::{HiDpi::*, WindowsAndMessaging::*},
};
#[path = "support/process.rs"]
mod process;
#[path = "support/product.rs"]
mod product;
#[path = "support/tile_product_window.rs"]
mod window;
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
struct Fixture {
    root: PathBuf,
    deadline: Instant,
}
impl Fixture {
    fn mark(&self, name: &str, value: Value) {
        let pending = self.root.join(format!("{name}.pending"));
        fs::write(&pending, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::rename(pending, self.root.join(name)).unwrap();
    }
    fn wait(&self, child: &mut window::Owned) {
        window::pump();
        assert!(
            Instant::now() < self.deadline,
            "visible Tile deadline: {}",
            self.root.display()
        );
        assert!(
            child.child.try_wait().unwrap().is_none(),
            "product exited: {}",
            self.root.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    fn request(&self, child: &mut window::Owned, name: &str) -> Value {
        let path = self.root.join(format!("product-request-{name}.json"));
        while !path.exists() {
            self.wait(child);
        }
        let data = fs::read(path).unwrap();
        assert!(data.len() < 4096);
        serde_json::from_slice(&data).unwrap()
    }
}
#[test]
#[ignore = "explicit physical visible ordinary Tile EXE gate; isolated Loom fixture and short owned full-screen color patterns"]
fn ordinary_tile_product_visible_pixels_and_stop() {
    run(&[
        "running", "rotated", "black", "resumed", "cropped", "frozen",
    ]);
}
#[test]
#[ignore = "explicit physical visible image Live Art ordinary EXE lifecycle gate"]
fn ordinary_tile_product_mixed_visible_pixels_and_stop() {
    run(&["mixed-first", "mixed-updated", "frozen", "black", "resumed"]);
}
fn run(stages: &[&str]) {
    assert_eq!(std::env::var("OTTER_TILE_VISIBLE_GATE").as_deref(), Ok("1"));
    let external_activation = std::env::var("OTTER_TILE_VISIBLE_ACTIVATE").as_deref() == Ok("1");
    let input = PathBuf::from(std::env::var_os("OTTER_TILE_JOINT_INPUT").unwrap());
    let root = input.parent().unwrap().to_owned();
    assert!(root.is_absolute());
    assert!(root.canonicalize().unwrap().starts_with(
        PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .canonicalize()
            .unwrap()
    ));
    let data = fs::read(&input).unwrap();
    assert!(data.len() < 65536);
    let value: Value = serde_json::from_slice(&data).unwrap();
    let origin = value["origin"].as_str().unwrap();
    let url = url::Url::parse(origin).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let f = Fixture {
        root,
        deadline: Instant::now() + Duration::from_secs(30),
    };
    let profile = f.root.join("native-profile");
    assert!(profile.is_dir());
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let product =
        product::Product::resolve(std::path::Path::new(env!("CARGO_BIN_EXE_otter-native")))
            .unwrap();
    let outputs = process::run(
        Command::new(product.path())
            .arg("--tile-outputs")
            .env("OTTER_NATIVE_PROFILE_DIR", &profile),
        Duration::from_secs(3),
    )
    .unwrap();
    assert!(outputs.status.success());
    let inventory: Value = serde_json::from_slice(&outputs.stdout).unwrap();
    let output = inventory.as_array().unwrap().first().unwrap();
    let id = output["outputId"].as_str().unwrap();
    let rect = [
        output["x"].as_i64().unwrap() as i32,
        output["y"].as_i64().unwrap() as i32,
        output["width"].as_u64().unwrap() as i32,
        output["height"].as_u64().unwrap() as i32,
    ];
    assert!((4..=8192).contains(&rect[2]) && (4..=8192).contains(&rect[3]));
    assert!(i64::from(rect[2]) * i64::from(rect[3]) <= 16_777_216);
    let mut child = window::Owned::new(
        Command::new(product.path())
            .args(["--tile-output", id, "--loom", origin, "--profile"])
            .arg(&profile)
            .env("OTTER_NATIVE_PROFILE_DIR", &profile)
            .stdin(Stdio::null())
            .stdout(fs::File::create(f.root.join("ordinary-tile.stdout.log")).unwrap())
            .stderr(fs::File::create(f.root.join("ordinary-tile.stderr.log")).unwrap())
            .creation_flags(0x08000000)
            .spawn()
            .unwrap(),
    );
    let hwnd = loop {
        if let Some(h) = window::find(child.child.id()) {
            break h;
        }
        f.wait(&mut child);
    };
    f.mark("product-window.json",json!({"pid":child.child.id(),"output":output,"window":format!("{:?}",hwnd),"ordinaryOutputExe":true,"noUserInputInjection":true,"driverNoFocusOrZOrderApi":!external_activation,"noFocusOrZOrderChange":!external_activation,"externalActivationRequired":external_activation}));
    let _launcher = external_activation.then(window::Launcher::new);
    if external_activation {
        // 外部激活/刷新完成后才确认ACK，避免其尚在验证启动器时并发转交前台。
        // ACK只同步测试时序，真实前台及自有PID/类断言仍由transfer独立核验。
        let nonce = format!("{}:{:?}", std::process::id(), _launcher.as_ref().unwrap().0);
        f.mark(
            "product-activation-ready.json",
            json!({"nonce":nonce,"driverPid":std::process::id(),"productPid":child.child.id()}),
        );
        let ack = f.root.join("product-activation-ack.json");
        while !ack.exists() {
            f.wait(&mut child);
        }
        assert!(fs::metadata(&ack).unwrap().len() <= 4096);
        let ack: Value = serde_json::from_slice(&fs::read(ack).unwrap()).unwrap();
        assert_eq!(ack["nonce"], nonce);
        assert_eq!(ack["driverPid"], std::process::id());
        assert_eq!(ack["productPid"], child.child.id());
        assert_eq!(ack["activatedBy"], "computerUseReturnedAppPath");
        while !_launcher.as_ref().unwrap().transfer(hwnd, child.child.id()) {
            f.wait(&mut child);
        }
        f.mark("product-activated.json",json!({"observedOwnedForeground":true,"observedOwnedLauncher":true,"driverNoActivationApi":false,"externalActivationRequested":true,"externalActivationCompletedBeforeTransfer":true,"mode":"externalComputerUseLauncherThenOwnedChildTransfer"}));
    }
    for &name in stages {
        let request = f.request(&mut child, name);
        let expected: [[u8; 3]; 4] = serde_json::from_value(request["rgb"].clone()).unwrap();
        let mut last = None;
        let mut last_probe = None;
        let actual = loop {
            let probe = window::samples(hwnd, child.child.id(), rect);
            if last_probe.as_ref() != Some(&probe) {
                // 只记录自己窗口的状态/像素，单槽覆盖；父期限先到也保留首个明确阻塞原因。
                f.mark(&format!("product-probe-{name}.json"),json!({"stage":name,"expectedRect":rect,"probe":format!("{probe:?}"),"foreignWindowPixelsRead":false}));
            }
            if let Ok(pixels) = &probe {
                last = Some(*pixels);
                if pixels
                    .iter()
                    .flatten()
                    .zip(expected.iter().flatten())
                    .all(|(a, b)| a.abs_diff(*b) <= 1)
                {
                    break *pixels;
                }
            }
            last_probe = Some(probe);
            assert!(
                Instant::now() < f.deadline,
                "visible stage {name}; last owned pixels: {last:?}; root {}",
                f.root.display()
            );
            f.wait(&mut child);
        };
        f.mark(&format!("product-pixels-{name}.json"),json!({"actualRgb":actual,"expectedRgb":expected,"requestRevision":request["revision"],"ordinaryOutputExe":true,"desktopCompositedPixels":true,"directSwapChainReadback":false,"occlusionChecked":true}));
    }
    f.request(&mut child, "stop");
    assert!(window::matches(hwnd, child.child.id()));
    unsafe {
        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
    }
    loop {
        if let Some(status) = child.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < f.deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(window::find(child.child.id()).is_none());
    f.mark("product-complete.json",json!({"ordinaryOutputExe":true,"normalCloseExit":true,"ownedWindowGone":true,"profileIsolated":true,"physicalCrossDevice":false,"multiMonitor":false}));
}
