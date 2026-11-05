#![cfg(windows)]
//! 独立验证测试进程 runner，不启动可见窗口或操作用户数据。
#[path = "support/process.rs"]
mod process;
use std::{io, process::Command, time::Duration};

fn shell(script: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    command
}
#[test]
fn drains_both_pipes_past_retention_cap_without_blocking_child() {
    let output = process::run(
        &mut shell("[Console]::Out.Write(('o' * 524288)); [Console]::Error.Write(('e' * 524288))"),
        Duration::from_secs(15),
    )
    .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, vec![b'o'; 256 * 1024]);
    assert_eq!(output.stderr, vec![b'e'; 256 * 1024]);
}
#[test]
fn timeout_preserves_partial_diagnostics_and_reaps_owned_child() {
    let error = process::run(&mut shell(
        "[Console]::Out.Write('before-timeout'); [Console]::Out.Flush(); Start-Sleep -Seconds 60"
    ), Duration::from_secs(3)).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("before-timeout"), "{error}");
}
