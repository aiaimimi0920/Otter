#![cfg(windows)]
//! 实际产品 EXE 的候选启动边界；可选真实门由显式 pinned fixture 供给，不默认激活。
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};
#[path = "support/shader_peer.rs"]
mod peer;
#[path = "support/process.rs"]
mod process;
fn run(args: &[&str]) -> Output {
    process::run(
        Command::new(env!("CARGO_BIN_EXE_otter-native")).args(args),
        Duration::from_secs(15),
    )
    .unwrap()
}
#[test]
fn shader_startup_invalid_options_fail_before_windows_capture_or_profile() {
    for args in [
        vec!["--shader-candidate"],
        vec!["--shader-candidate", ""],
        vec!["--shader-candidate", "--capture"],
        vec!["--shader-candidate", "a", "--shader-candidate", "b"],
        vec!["--shader-smoke-peer", "missing.json"],
        vec!["--shader-smoke-peer", "missing.json", "--smoke-test"],
        vec![
            "--shader-candidate",
            "a",
            "--shader-smoke-peer",
            "missing.json",
        ],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
#[test]
fn shader_startup_rejects_untrusted_candidate_in_every_product_mode() {
    for mode in ["--smoke-test", "--capture", "--capture-region", "--session"] {
        let mut args = vec![
            "--shader-candidate",
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/missing-shader-candidate",
            mode,
        ];
        if mode == "--session" {
            args.extend(["missing.json", "--session-check"]);
        }
        let output = run(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Shader candidate"));
    }
}
#[test]
fn shader_startup_private_workers_reject_config_without_reading_it() {
    for flag in [
        "--native-hlsl-worker",
        "--native-shader-worker",
        "--native-svg-worker",
        "--native-webp-worker",
        "--native-recovery-parent",
    ] {
        let output = run(&[
            flag,
            "--shader-candidate",
            "C:/missing",
            "--shader-smoke-peer",
            "C:/missing",
        ]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Shader candidate"));
    }
}
#[test]
fn retired_angle_worker_is_rejected_before_private_dispatch() {
    for args in [
        vec!["--native-shader-worker"],
        vec!["--native-hlsl-worker", "--native-shader-worker"],
        vec!["--native-shader-worker", "--settings", "missing.json"],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Legacy ANGLE worker is retired"));
    }
}
#[test]
fn production_library_cannot_load_angle_or_start_legacy_child() {
    use otter_native::{shader_angle as angle, shader_process};
    let directory = std::env::temp_dir();
    let fingerprint = angle::Fingerprint {
        bytes: 1,
        sha256: [0; 32],
    };
    let identity = angle::Identity {
        egl: fingerprint,
        gles: fingerprint,
    };
    // 集成测试链接非cfg(test)产品库；不借助产品CLI拒绝来掩盖公共API仍可加载。
    assert!(matches!(
        angle::Renderer::new(&directory, identity, 1, 1, angle::Backend::D3d11),
        Err(angle::Error::Library)
    ));
    let bundle = shader_process::TrustedBundle::new(&directory, identity).unwrap();
    let invocation = shader_process::Invocation {
        bundle: &bundle,
        backend: angle::Backend::D3d11,
        width: 1,
        height: 1,
        request: angle::Request {
            vertex: "legacy",
            fragment: "legacy",
            textures: &[],
            uniforms: &[],
        },
    };
    assert!(
        matches!(shader_process::render(invocation, &|| panic!("retired path must not execute")),
        Err(shader_process::Failure::Transport(error)) if error.kind() == std::io::ErrorKind::Unsupported)
    );
}
#[test]
#[ignore = "explicit actual product native GPU startup and cache integration"]
fn native_shader_product_default_fetch_commit_and_cached_rerender() {
    let identity = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
    let root = std::env::temp_dir().join(format!("otter-native-shader-{identity:?}"));
    fs::create_dir(&root).unwrap();
    let peer = peer::Peer::new();
    let manifest = root.join("peer.json");
    fs::write(&manifest, peer.manifest.to_string()).unwrap();
    let server = std::thread::spawn(move || peer.serve_native());
    let output = run(&[
        "--smoke-test",
        "--shader-smoke-peer",
        manifest.to_str().unwrap(),
    ]);
    let listener = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("native-shader-startup: native=true application=true default=true"));
    for iteration in ["iteration=0", "iteration=1"] {
        assert!(stdout
            .lines()
            .any(|line| line.contains("native-shader-product-smoke:")
                && line.contains(iteration)
                && line.contains("child=true committed=true png=true parent-loaded=false")));
    }
    assert!(stdout.contains("cache-hit=false first-fetch=true"));
    assert!(stdout.contains("cache-hit=true first-fetch=false"));
    assert!(stdout.contains("native-smoke:"));
    // 保留本次小型合成peer夹具；不清理用户目录或其他测试证据。
}
struct Fixture(PathBuf);
impl Fixture {
    fn private() -> Self {
        let path = PathBuf::from(
            std::env::var_os("OTTER_SHADER_PRIVATE_TEST_ROOT")
                .expect("Set fresh private test root outside shared ancestors"),
        );
        assert!(path.is_absolute() && !path.exists());
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn copy_candidate(&self, source: &Path) -> PathBuf {
        let target = self.0.join("候选 DLL");
        fs::create_dir(&target).unwrap();
        for name in ["libEGL.dll", "libGLESv2.dll"] {
            fs::copy(source.join(name), target.join(name)).unwrap();
        }
        let output = process::run(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", r#"
$ErrorActionPreference='Stop'; $user=[Security.Principal.WindowsIdentity]::GetCurrent().User
$acl=Get-Acl -LiteralPath $env:OTTER_SHADER_TEST_COPY; $acl.SetAccessRuleProtection($true,$false); foreach($rule in @($acl.Access)){ [void]$acl.RemoveAccessRuleSpecific($rule) }
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user,'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
Set-Acl -LiteralPath $env:OTTER_SHADER_TEST_COPY -AclObject $acl
foreach($name in 'libEGL.dll','libGLESv2.dll'){ $p=Join-Path $env:OTTER_SHADER_TEST_COPY $name; $f=Get-Acl -LiteralPath $p; $f.SetAccessRuleProtection($true,$false); foreach($rule in @($f.Access)){ [void]$f.RemoveAccessRuleSpecific($rule) }; $f.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user,'FullControl','Allow')); Set-Acl -LiteralPath $p -AclObject $f }
"#]).env("OTTER_SHADER_TEST_COPY", &target), Duration::from_secs(10)).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        target
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 安全祖先运行夹具可能位于用户私有目录；linshi 外一律保留，不自动删除。
        if !self
            .0
            .starts_with(Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi"))
        {
            return;
        }
        for file in ["libEGL.dll", "libGLESv2.dll"] {
            let _ = fs::remove_file(self.0.join("候选 DLL").join(file));
        }
        let _ = fs::remove_dir(self.0.join("候选 DLL"));
        let _ = fs::remove_file(self.0.join("peer.json"));
        let _ = fs::remove_dir(&self.0);
    }
}
#[test]
#[ignore = "optional historical pinned DLL fixture; retired product path must reject even trusted files"]
fn shader_product_exe_rejects_retired_trusted_candidate_without_network() {
    let source =
        std::env::var_os("OTTER_SHADER_CANDIDATE").expect("Set reviewed v332/v333 payload path");
    let fixture = Fixture::private();
    let candidate = fixture.copy_candidate(Path::new(&source));
    let peer = peer::Peer::new();
    let manifest = fixture.0.join("peer.json");
    fs::write(&manifest, peer.manifest.to_string()).unwrap();
    let output = run(&[
        "--smoke-test",
        "--shader-candidate",
        candidate.to_str().unwrap(),
        "--shader-smoke-peer",
        manifest.to_str().unwrap(),
    ]);
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Shader candidate is retired"));
    peer.assert_unused();
    // 产品不再打开候选或启动旧 child；历史夹具应仍可移动。
    let moved = candidate.with_extension("moved");
    fs::rename(&candidate, &moved).unwrap();
    fs::rename(&moved, &candidate).unwrap();
    println!("retired-shader-product-verified: connections=0 children=0 locks-released=true");
}
