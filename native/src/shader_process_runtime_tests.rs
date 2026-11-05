//! 显式真实平台门：同一父侧发布门分别启动测试 child 和实际产品私有入口。
use super::*;
use crate::{
    bounded_worker_process as process,
    shader_angle::{Texture, Uniform},
};
use std::{os::windows::io::OwnedHandle, path::PathBuf};
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

fn bundle() -> TrustedBundle {
    TrustedBundle::new(
        &PathBuf::from(
            std::env::var_os("OTTER_SHADER_ANGLE_FIXTURE")
                .expect("Set OTTER_SHADER_ANGLE_FIXTURE to the isolated verified two-DLL fixture"),
        ),
        angle::tests::fixture_identity(),
    )
    .unwrap()
}
fn product_exe() -> PathBuf {
    let current = std::env::current_exe().unwrap();
    let profile = current.parent().unwrap().parent().unwrap();
    let path = profile.join("otter-native.exe");
    assert!(
        path.is_file(),
        "Build the same target/profile otter-native product EXE first"
    );
    path
}
fn spawn(endpoint: OwnedHandle, deadline: Instant, product: bool) -> io::Result<process::Child> {
    if product {
        process::start_at(
            endpoint,
            deadline,
            &|| false,
            process::Policy {
                suffix: " --native-shader-worker",
                maximum: CHILD_LIMIT,
                pool: &CAPACITY,
                environment: process::Environment::SystemRoot,
            },
            &product_exe(),
        )
    } else {
        process::start(
            endpoint,
            deadline,
            &|| false,
            process::Policy {
                suffix: " --exact shader_worker::tests::shader_worker_entry --ignored --nocapture",
                maximum: CHILD_LIMIT,
                pool: &CAPACITY,
                environment: process::Environment::SystemRoot,
            },
        )
    }
}
fn execute(value: Invocation<'_>, product: bool) -> (Result<Frame, Failure>, usize) {
    let mut probe = None;
    let result = client::run(
        value,
        Instant::now() + TIMEOUT,
        &|| false,
        |endpoint, deadline| {
            let child = spawn(endpoint, deadline, product)?;
            probe = Some(child.probe()?);
            Ok(child)
        },
    );
    let probe = probe.expect("real child was started");
    assert!(
        probe.exited(),
        "Shader API returned before child exit/reaping"
    );
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT, "peak={peak}");
    drop(
        CAPACITY
            .reserve(CHILD_LIMIT)
            .expect("child capacity lease must be returned"),
    );
    assert_parent_has_no_angle();
    (result, peak)
}
fn assert_parent_has_no_angle() {
    for name in [w!("libEGL.dll"), w!("libGLESv2.dll")] {
        assert!(
            unsafe { GetModuleHandleW(name) }.is_err(),
            "parent must not load ANGLE"
        );
    }
}
#[test]
#[ignore = "requires explicit verified ANGLE/browser fixtures and freshly built same-profile product EXE"]
fn actual_shader_children_and_product_exe_match_browser_contract_and_reap() {
    let bundle = bundle();
    let cases = angle::contract_tests::cases();
    assert_parent_has_no_angle();
    for product in [false, true] {
        for backend in [Backend::D3d11, Backend::Warp] {
            for case in &cases {
                let textures = case
                    .textures
                    .iter()
                    .map(|item| Texture {
                        name: &item.name,
                        width: item.width,
                        height: item.height,
                        rgba: &item.rgba,
                    })
                    .collect::<Vec<_>>();
                let uniforms = case
                    .uniforms
                    .iter()
                    .map(|(name, value)| Uniform {
                        name,
                        value: *value,
                    })
                    .collect::<Vec<_>>();
                let (result, peak) = execute(
                    Invocation {
                        bundle: &bundle,
                        backend,
                        width: case.width,
                        height: case.height,
                        request: Request {
                            vertex: &case.vertex,
                            fragment: &case.fragment,
                            textures: &textures,
                            uniforms: &uniforms,
                        },
                    },
                    product,
                );
                let frame = result.unwrap();
                assert_eq!(frame.rgba.len(), case.expected_raw_rgba.len());
                let mut maximum = 0;
                for (index, (actual, expected)) in
                    frame.rgba.iter().zip(&case.expected_raw_rgba).enumerate()
                {
                    let delta = actual.abs_diff(*expected);
                    let tolerance =
                        u8::from(case.name == "loom-inverted-lut-tone" && index % 4 != 3);
                    assert!(
                        delta <= tolerance,
                        "product={product} backend={backend:?} case={} byte={index}",
                        case.name
                    );
                    maximum = maximum.max(delta);
                }
                eprintln!("native-shader-child-contract: product={product} backend={backend:?} case={} max-byte-delta={maximum} peak-job-bytes={peak} reaped=true parent-angle=false", case.name);
            }
        }
    }
}
#[test]
#[ignore = "requires explicit verified ANGLE fixture and freshly built same-profile product EXE; actual 4K allocation"]
fn actual_shader_product_4k_identity_alpha_orientation_and_peak() {
    let bundle = bundle();
    let (width, height) = (3840u32, 2160u32);
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        let x = index % width as usize;
        let y = index / width as usize;
        pixel.copy_from_slice(&[
            x as u8,
            y as u8,
            (x ^ y) as u8,
            [0, 64, 128, 255][index % 4],
        ]);
    }
    let textures = [Texture {
        name: "input",
        width,
        height,
        rgba: &pixels,
    }];
    let uniforms = [Uniform {
        name: "gain",
        value: 1.0,
    }];
    for backend in [Backend::D3d11, Backend::Warp] {
        let start = Instant::now();
        let (result, peak) = execute(
            Invocation {
                bundle: &bundle,
                backend,
                width,
                height,
                request: Request {
                    vertex: angle::tests::VERTEX,
                    fragment: angle::tests::FRAGMENT,
                    textures: &textures,
                    uniforms: &uniforms,
                },
            },
            true,
        );
        eprintln!(
            "native-shader-child-4k-probe: backend={backend:?} peak-job-bytes={peak} success={}",
            result.is_ok()
        );
        let frame = result.unwrap();
        assert_eq!((frame.width, frame.height), (width, height));
        assert_eq!(frame.rgba.len(), pixels.len());
        // 错误只报告首个字节，不把两份 4K 私有图片倾倒到测试日志。
        let first = frame
            .rgba
            .iter()
            .zip(&pixels)
            .enumerate()
            .find(|(_, (actual, expected))| actual != expected)
            .map(|(index, (actual, expected))| (index, *actual, *expected));
        assert_eq!(first, None, "4K top-down raw RGBA including alpha=0 RGB");
        eprintln!("native-shader-child-4k: backend={backend:?} bytes={} peak-job-bytes={peak} elapsed-ms={} identity-alpha-orientation=true reaped=true", pixels.len(), start.elapsed().as_millis());
    }
}
#[test]
#[ignore = "requires explicit verified ANGLE fixture and freshly built same-profile product EXE"]
fn actual_shader_child_compile_and_fingerprint_failures_are_typed_and_reaped() {
    let bundle = bundle();
    let mut wrong_identity = bundle.identity;
    wrong_identity.egl.sha256[0] ^= 1;
    let wrong = TrustedBundle::new(&bundle.directory, wrong_identity).unwrap();
    let pixels = [64, 32, 16, 128];
    let textures = [Texture {
        name: "input",
        width: 1,
        height: 1,
        rgba: &pixels,
    }];
    for product in [false, true] {
        let request = || Request {
            vertex: angle::tests::VERTEX,
            fragment: "#version 300 es\nnot a shader",
            textures: &textures,
            uniforms: &[],
        };
        for (candidate, compile) in [(&bundle, true), (&wrong, false)] {
            let (result, peak) = execute(
                Invocation {
                    bundle: candidate,
                    backend: Backend::Warp,
                    width: 1,
                    height: 1,
                    request: request(),
                },
                product,
            );
            if compile {
                assert!(matches!(
                    result,
                    Err(Failure::Backend(angle::Error::Compile(_)))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(Failure::Backend(angle::Error::Library))
                ));
            }
            eprintln!("native-shader-child-failure: product={product} compile={compile} peak-job-bytes={peak} typed=true reaped=true");
        }
    }
}

#[path = "shader_image_runtime_tests.rs"]
mod image;
