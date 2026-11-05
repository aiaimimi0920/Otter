#![cfg(windows)]
//! 真实 EXE 的隐藏会话入口；所有文件仅在 linshi，超时只终止本测试创建的子进程。
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
#[path = "support/process.rs"]
mod process;
struct Fixture(PathBuf);
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-session-startup",
        );
        std::fs::create_dir_all(&root).unwrap();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // Windows 时钟分辨率不保证并发调用唯一；序号隔离线程，create_new 保护历史文件。
        for _ in 0..32 {
            let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "会话 空白-{}-{stamp}-{sequence}.json",
                std::process::id()
            ));
            let mut file = match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("Cannot create session fixture: {error}"),
            };
            let fixture = Self(path);
            file.write_all(bytes).unwrap();
            return fixture;
        }
        panic!("Session fixture collision retry exhausted");
    }
}
#[test]
fn concurrent_fixture_paths_are_distinct_and_contents_are_isolated() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
    let workers: Vec<_> = (0u8..16)
        .map(|index| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let fixture = Fixture::new(&[index]);
                assert_eq!(std::fs::read(&fixture.0).unwrap(), [index]);
                fixture
            })
        })
        .collect();
    let fixtures: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    let paths: std::collections::HashSet<_> = fixtures.iter().map(|fixture| &fixture.0).collect();
    assert_eq!(paths.len(), 16);
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn run(fixture: &Fixture, scale: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
    command.arg("--session").arg(&fixture.0).args([
        "--session-origin",
        "-1920,-200",
        "--session-scale",
        scale,
        "--session-check",
    ]);
    process::run(&mut command, Duration::from_secs(30)).unwrap()
}
fn run_embedded(fixture: &Fixture) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
    command
        .arg("--session")
        .arg(&fixture.0)
        .arg("--session-check");
    process::run(&mut command, Duration::from_secs(30)).unwrap()
}

#[test]
fn loaded_recycle_is_pruned_without_user_input_or_overwriting_source() {
    let entries: Vec<_> = (0..20)
        .map(|i| {
            serde_json::json!({"entryId":format!("e-{i}"),
        "sourceStickerId":"a","createdAt":"legacy","snapshot":{"id":"a","x":0,"y":0,"w":32,"h":32}})
        })
        .collect();
    let bytes =
        serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],"recycleBin":entries}))
            .unwrap();
    let fixture = Fixture::new(&bytes);
    let output = run(&fixture, "1");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("native-session-recycle: entries=15"),
        "{text}"
    );
    assert!(
        text.contains("native-session-save-state: Unsaved"),
        "{text}"
    );
    assert_eq!(std::fs::read(&fixture.0).unwrap(), bytes);
}

#[test]
fn embedded_context_positions_real_window_and_explicit_context_overrides_it() {
    let fixture = Fixture::new(br#"{"stickers":[{"id":"one","x":10,"y":20,"w":200,"h":100}],"links":[],"otterNativeCoordinateSpace":{"version":1,"origin":[-1920,-200],"scale":1.5}}"#);
    for (output, placement) in [
        (
            run_embedded(&fixture),
            "left=-1905 top=-170 width=300 height=150",
        ),
        (
            run(&fixture, "1"),
            "left=-1910 top=-180 width=200 height=100",
        ),
    ] {
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(text.contains(placement), "{text}");
        assert!(
            text.contains("native-session-save-state: Unchanged"),
            "{text}"
        );
    }
}

#[test]
fn legacy_without_context_and_future_metadata_fail_without_publication() {
    for bytes in [
        br#"{"stickers":[],"links":[]}"#.as_slice(),
        br#"{"stickers":[],"links":[],"otterNativeCoordinateSpace":{"version":2,"origin":[0,0],"scale":1}}"#.as_slice(),
    ] {
        let fixture = Fixture::new(bytes);
        let output = run_embedded(&fixture);
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("native-session-check:"));
    }
}
#[test]
fn real_exe_restores_file_offline_without_showing_windows() {
    let fixture =
        Fixture::new(br#"{"stickers":[{"id":"one","x":10,"y":20,"w":64,"h":32}],"links":[]}"#);
    let output = run(&fixture, "1.5");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("native-session-check: windows=1 annotations=0"),
        "{text}"
    );
    assert!(text.contains("offline=true hidden=true"), "{text}");
    assert!(
        text.contains("native-session-save-state: Unchanged"),
        "{text}"
    );
}
#[test]
fn real_exe_restores_empty_session_with_working_host() {
    let fixture = Fixture::new(br#"{"stickers":[],"links":[],"unknown":true}"#);
    let output = run(&fixture, "1");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("windows=1 annotations=0"));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("native-session-save-state: Unchanged")
    );
}

#[test]
fn missing_original_recovers_unique_copy_in_real_exe_without_moving_files() {
    let bytes = br#"{"documentRevision":7,"stickers":[],"links":[]}"#;
    let fixture = Fixture::new(bytes);
    let mut name = fixture.0.file_name().unwrap().to_os_string();
    name.push(".otter-recovery-01234567-89AB-CDEF-0123-456789ABCDEF.json");
    let recovery = fixture.0.with_file_name(name);
    std::fs::rename(&fixture.0, &recovery).unwrap();
    let output = run(&fixture, "1");
    let retained = std::fs::read(&recovery).unwrap();
    std::fs::remove_file(&recovery).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("native-session-recovery: loaded=true save-as-required=true"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("native-session-save-state: Unsaved"));
    assert!(!fixture.0.exists());
    assert_eq!(retained, bytes);
}
#[test]
fn bad_json_and_invalid_coordinate_context_fail_without_success_receipt() {
    let fixture = Fixture::new(b"invalid-json");
    for scale in ["1", "NaN"] {
        let output = run(&fixture, scale);
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("native-session-check:"));
    }
}

#[test]
fn restored_image_text_blur_and_minified_canvas_rebuild_in_real_exe() {
    // 合成的 2×2 全白 RGBA PNG；不依赖私有像素构造器或用户文件。
    let base64 = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAADklEQVR4nGP4DwUMMAYAj4IP8TylVlEAAAAASUVORK5CYII=";
    for minified in [false, true] {
        let session = serde_json::json!({"stickers":[{
            "id":"rendered", "x":10,"y":20,"w":16,"h":12,
            "minified":minified, "savedRect":{"x":0,"y":0,"w":32,"h":24},
            "cropOffset":{"x":4,"y":2}, "src":format!("data:image/png;base64,{base64}"),
            "annotationState":{"serialCounter":0,"elements":[
                {"id":"shape","type":"rect","x":2,"y":2,"w":8,"h":6,"zIndex":0,"style":{"width":1,"color":"#f00","fill":"#0f0"}},
                {"id":"text","type":"text","x":3,"y":10,"text":"恢复","fontSize":8,"zIndex":1,"style":{"width":1,"color":"#fff"}},
                {"id":"blur","type":"blur","points":[{"x":8,"y":6}],"brushWidth":4,"strength":2,"zIndex":2,"style":{"width":1,"color":"#fff"}}
            ]}
        }],"links":[]});
        let fixture = Fixture::new(&serde_json::to_vec(&session).unwrap());
        let output = run(&fixture, "1.5");
        assert!(
            output.status.success(),
            "minified={minified}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("windows=1 annotations=3"), "{text}");
        assert!(
            text.contains("native-session-render-check: paints=2 device-recreated=true"),
            "{text}"
        );
    }
}
