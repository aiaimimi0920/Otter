#![cfg(windows)]
//! 真实 EXE 的外部图片准入；只读合成源文件，所有夹具和清理严格限定在 linshi。
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};
#[path = "support/process.rs"]
mod process;
const BASE: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-assets-check";
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 14, 73, 68, 65, 84, 120, 156, 99, 248, 15, 5, 12, 48, 6, 0,
    143, 130, 15, 241, 60, 165, 86, 81, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
struct Fixture {
    root: PathBuf,
    files: Vec<(PathBuf, Vec<u8>)>,
    directories: Vec<PathBuf>,
}
impl Fixture {
    fn new() -> Self {
        fs::create_dir_all(BASE).unwrap();
        let root = Path::new(BASE).join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        fs::create_dir(&root).unwrap();
        Self {
            root,
            files: Vec::new(),
            directories: Vec::new(),
        }
    }
    fn directory(&mut self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir(&path).unwrap();
        self.directories.push(path.clone());
        path
    }
    fn file(&mut self, path: PathBuf, bytes: &[u8]) -> PathBuf {
        assert!(path.starts_with(&self.root));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        self.files.push((path.clone(), bytes.to_vec()));
        file.write_all(bytes).unwrap();
        path
    }
    fn session(&mut self, index: usize, reference: &Path, original: &Path) -> PathBuf {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "stickers":[{"id":"external-image","x":10,"y":20,"w":200,"h":100,
                "src":reference.to_str().unwrap(), "filePath":original.to_str().unwrap()}], "links":[],
            "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1}
        }))
        .unwrap();
        self.file(self.root.join(format!("session-{index}.json")), &bytes)
    }
    fn assert_unchanged(&self) {
        for (path, bytes) in &self.files {
            assert_eq!(fs::read(path).unwrap(), *bytes);
        }
    }
    fn cleanup(&mut self) {
        assert_eq!(self.root.parent(), Some(Path::new(BASE)));
        for (path, _) in &self.files {
            fs::remove_file(path).unwrap();
        }
        self.files.clear();
        for path in self.directories.iter().rev() {
            fs::remove_dir(path).unwrap();
        }
        self.directories.clear();
        fs::remove_dir(&self.root).unwrap();
        assert!(!self.root.exists());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 仅删除 create_new 成功登记的文件与本夹具目录；不递归扫描或跟随目录链接。
        assert_eq!(self.root.parent(), Some(Path::new(BASE)));
        for (path, _) in &self.files {
            let _ = fs::remove_file(path);
        }
        for path in self.directories.iter().rev() {
            let _ = fs::remove_dir(path);
        }
        let _ = fs::remove_dir(&self.root);
    }
}
fn run(session: &Path, assets: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
    command.arg("--session").arg(session).arg("--session-check");
    if let Some(assets) = assets {
        command.arg("--session-assets").arg(assets);
    }
    process::run(&mut command, Duration::from_secs(30)).unwrap()
}

#[test]
fn external_images_require_exact_directory_and_remain_read_only_in_real_exe() {
    let mut fixture = Fixture::new();
    let assets = fixture.directory("图片 目录");
    let sibling = fixture.directory("图片 目录-extra");
    let nested = fixture.directory("图片 目录/nested");
    let image = fixture.file(assets.join("白色 image.png"), PNG);
    let other = fixture.file(sibling.join("other.png"), PNG);
    let child = fixture.file(nested.join("nested.png"), PNG);
    let broken = fixture.file(assets.join("broken.png"), b"not an image");
    let cases = [
        (image.clone(), None, false),
        (image.clone(), Some(assets.as_path()), true),
        (image.clone(), Some(sibling.as_path()), false),
        (other.clone(), Some(assets.as_path()), false),
        (child, Some(assets.as_path()), false),
        (
            assets.join("../图片 目录-extra/other.png"),
            Some(assets.as_path()),
            false,
        ),
        (broken, Some(assets.as_path()), false),
    ];
    for (index, (reference, root, accepted)) in cases.into_iter().enumerate() {
        let session = fixture.session(index, &reference, &image);
        let output = run(&session, root);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.status.success(),
            accepted,
            "case {index}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if accepted {
            assert!(stdout.contains("windows=1 annotations=0"), "{stdout}");
            assert!(
                stdout.contains("native-session-render-check: paints=2 device-recreated=true"),
                "{stdout}"
            );
            assert!(
                stdout.contains("native-session-save-state: Unchanged"),
                "{stdout}"
            );
        } else {
            assert!(!stdout.contains("native-session-check:"), "{stdout}");
        }
        fixture.assert_unchanged();
    }
    // EXE 退出后的图片和授权目录必须可移动；验证 OS 句柄未被外部残留进程持有。
    let moved = assets.join("moved.png");
    fs::rename(&image, &moved).unwrap();
    fs::rename(&moved, &image).unwrap();
    let moved = fixture.root.join("moved-assets");
    fs::rename(&assets, &moved).unwrap();
    fs::rename(&moved, &assets).unwrap();
    fixture.assert_unchanged();
    fixture.cleanup();
}
