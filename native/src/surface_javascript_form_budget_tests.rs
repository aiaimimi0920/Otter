//! 同一生产Engine的定向耗时探针；只记录固定夹具，不修改250ms期限或输出私有异常。
use super::*;
use crate::{surface_javascript::fixture, surface_viewport::Viewport};
use windows::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentThread, GetThreadTimes},
};

fn cpu_ticks() -> u64 {
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
        .unwrap();
    }
    let ticks =
        |value: FILETIME| u64::from(value.dwLowDateTime) | (u64::from(value.dwHighDateTime) << 32);
    ticks(kernel) + ticks(user)
}

fn observe(label: &str, round: u32, source: &str) {
    let mut engine = Engine::new().unwrap();
    let cpu = cpu_ticks();
    let started = Instant::now();
    let result = engine.apply(
        Command::Mount {
            source: source.into(),
            snapshot: fixture::snapshot(3, 42),
            resources: fixture::resources(),
        },
        Some(Viewport::new([320, 200], 96).unwrap()),
    );
    let wall_us = started.elapsed().as_micros();
    let cpu_100ns = cpu_ticks() - cpu;
    let report = engine.diagnostic();
    let status = result.as_ref().ok().and_then(|out| {
        out.scene["children"][0]["props"]["text"]
            .as_str()
            .map(str::to_owned)
    });
    println!(
        "native-form-value-profile: {}",
        serde_json::json!({"label":label,"round":round,"wallUs":wall_us,
            "threadCpu100ns":cpu_100ns,"operationSucceeded":result.is_ok(),
            "status":status,"diagnostic":report})
    );
    if let Err(error) = result {
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
#[ignore = "fixed diagnostic samples only; explicit run before/after changes, not a passing budget gate"]
fn javascript_form_value_profile_original_mount_and_boundary_writes() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for round in 0..3 {
        observe(
            "original-input-mount",
            round,
            include_str!("../tests/fixtures/javascript-native-input.js"),
        );
    }
    for (label, expression, reject) in [
        ("ascii-boundary", "'x'.repeat(48*1024)", false),
        ("ascii-overflow", "'x'.repeat(48*1024+1)", true),
        ("bmp-boundary", "'中'.repeat(16*1024)", false),
        ("astral-boundary", "'🙂'.repeat(12*1024)", false),
        ("mixed-newlines", "'a\\r\\n'.repeat(21845)+'a'", false),
        ("newlines-only", "'\\n'.repeat(65536)", false),
    ] {
        let source = format!(
            r#"NeuroSurface.define({{mount({{root}}){{
          root.innerHTML='<p></p><input value="seed">';
          const field=root.children[1], raw={expression}, begin=performance.now();
          let rejected=false;try{{field.value=raw;}}catch{{rejected=true;}}
          const elapsed=performance.now()-begin;
          if(rejected!=={reject})throw Error('profile result');
          const length=field.value.length;field.value='seed';
          root.children[0].textContent=JSON.stringify({{elapsedMs:elapsed,length,rejected}});
        }}}});"#
        );
        for round in 0..3 {
            observe(label, round, &source);
        }
    }
}
