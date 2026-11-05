//! 产品裸私有入口不能绕过 Job/pipe 门，也不能混合正常 profile/UI 参数。
#![cfg(windows)]
use std::{process::Command, time::Duration};
#[path = "support/process.rs"]
mod process;
#[test]
fn javascript_private_product_entry_requires_job_before_reading_input_or_profile() {
    let output = process::run(
        Command::new(env!("CARGO_BIN_EXE_otter-native")).arg("--native-javascript-worker"),
        Duration::from_secs(5),
    )
    .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("native-smoke:"));
}
#[test]
fn javascript_private_product_mode_rejects_mixed_flags_without_creating_profile() {
    let path = std::env::temp_dir().join(format!("otter-js-unused-profile-{}", std::process::id()));
    assert!(!path.exists());
    for flag in [
        "--settings",
        "--smoke-test",
        "--native-shader-worker",
        "--native-svg-worker",
        "--native-javascript-worker",
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
        command.args(["--native-javascript-worker", flag]);
        if flag == "--settings" {
            command.arg(&path);
        }
        let output = process::run(&mut command, Duration::from_secs(5)).unwrap();
        assert!(!output.status.success(), "{flag}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("native-smoke:"));
        assert!(!path.exists());
    }
}
