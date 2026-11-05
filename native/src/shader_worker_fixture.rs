//! 测试专用固定 child；以 wire 内 mode 选择故障，不执行 shader，也不访问 DLL/profile。
use super::*;
use std::{io::Write, time::Duration};
#[test]
#[ignore = "private Shader fault peer invoked by bounded parent tests"]
fn shader_fault_entry() {
    crate::bounded_worker_process::require_limits(CHILD_LIMIT).unwrap();
    let environment = std::env::vars_os().collect::<Vec<_>>();
    assert_eq!(
        environment.len(),
        1,
        "Shader child must not inherit parent environment"
    );
    assert!(environment[0]
        .0
        .to_str()
        .unwrap()
        .eq_ignore_ascii_case("SystemRoot"));
    assert!(std::path::Path::new(&environment[0].1).is_dir());
    let mut pipe = crate::live_recovery_pipe::Pipe::adopt(
        std::io::stdin().as_handle().try_clone_to_owned().unwrap(),
    )
    .unwrap();
    let mut channel = pipe.io(Some(Instant::now() + TIMEOUT), None);
    let request = wire::read_request(&mut channel).unwrap();
    let mode = request.with_invocation(|value| {
        value
            .request
            .uniforms
            .iter()
            .find(|uniform| uniform.name == "mode")
            .unwrap()
            .value as u32
    });
    if mode == 5 {
        std::thread::sleep(Duration::from_secs(60));
        std::process::exit(1);
    }
    if mode == 6 {
        wire::write_response(
            &mut channel,
            &Err(crate::shader_angle::Error::Compile(
                "fixture compile diagnostic".into(),
            )),
        )
        .unwrap();
        std::process::exit(0);
    }
    let mut bytes = Vec::new();
    wire::write_response(
        &mut bytes,
        &Ok(crate::shader_angle::Frame {
            width: 1,
            height: 1,
            rgba: vec![64, 32, 16, 128].into(),
        }),
    )
    .unwrap();
    if mode == 1 {
        bytes[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
    }
    if mode == 2 {
        bytes.truncate(26);
    }
    if mode == 3 {
        bytes.push(1);
    }
    if mode == 9 {
        bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
    }
    if mode == 10 {
        bytes[4..8].copy_from_slice(&11u32.to_le_bytes());
    }
    channel.write_all(&bytes).unwrap();
    if mode == 7 {
        std::thread::sleep(Duration::from_secs(60));
    }
    std::process::exit(if mode == 4 { 7 } else { 0 });
}
