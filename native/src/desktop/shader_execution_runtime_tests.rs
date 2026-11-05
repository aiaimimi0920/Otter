//! 显式真实 ANGLE child → 同一异步任务集合 → 本地正式端口；不使用用户数据或正式安装 bundle。
use super::*;
use crate::{
    shader_angle::{self as angle, Backend},
    shader_process::{OwnedInvocation, OwnedTexture, TrustedBundle},
};
pub(crate) fn invocation(backend: Backend) -> OwnedInvocation {
    let directory =
        std::env::var_os("OTTER_SHADER_ANGLE_FIXTURE").expect("Set verified ANGLE fixture");
    let frame = oracle::matrix();
    OwnedInvocation {
        bundle: Arc::new(
            TrustedBundle::new(
                std::path::Path::new(&directory),
                angle::tests::fixture_identity(),
            )
            .unwrap(),
        ),
        backend,
        width: frame.width,
        height: frame.height,
        vertex: angle::tests::VERTEX.into(),
        fragment: angle::tests::FRAGMENT.into(),
        textures: vec![OwnedTexture {
            name: "input".into(),
            width: frame.width,
            height: frame.height,
            rgba: frame.rgba,
        }],
        uniforms: vec![("gain".into(), 1.0)],
    }
}
#[test]
#[ignore = "requires explicit verified ANGLE fixture; real bounded child and hardware/WARP"]
fn local_shader_real_child_commits_exact_png_and_display_through_shared_tasks() {
    let _serial = SERIAL.lock().unwrap();
    for backend in [Backend::D3d11, Backend::Warp] {
        let editor = Editor::default();
        let (notify, received) = std::sync::mpsc::channel();
        let mut tasks = Tasks::with_notify(Arc::new(move || {
            let _ = notify.send(());
        }));
        let mut registry = Requests::default();
        let request = registry.begin(id("local"), id("real-shader")).unwrap();
        let shader = invocation(backend);
        let input_lifetime = shader.textures[0].rgba.downgrade();
        tasks
            .start_shader(
                shader::Execution {
                    unit: id("local"),
                    request: request.clone(),
                    inputs: PreparedBatch::empty_for_test(id("local"), &editor),
                    shader: shader.into(),
                },
                &registry,
            )
            .unwrap();
        let mut results = Results::new(id("local"));
        results.begin(request);
        received.recv_timeout(Duration::from_secs(20)).unwrap();
        assert!(
            input_lifetime.upgrade().is_none(),
            "notification precedes input release"
        );
        let mut report = wait(&mut tasks, &registry);
        report
            .commit(&mut registry, &mut results, |_| Some(&editor))
            .unwrap();
        let source = results.image_source("output_image").unwrap();
        let expected = shader_image::prepare(oracle::matrix(), &AtomicBool::new(false)).unwrap();
        assert_eq!(source.image.pixels, expected.image.pixels);
        assert_eq!(
            oracle::hash(&oracle::decode_rgba(
                source.image.encoded_png().unwrap(),
                256,
                256
            )),
            oracle::STRAIGHT_SHA256
        );
        assert!(source
            .image
            .pixels
            .same_storage(&results.image_source("output").unwrap().image.pixels));
        assert!(registry.is_idle());
        assert!(tasks.is_empty());
        drop(
            crate::shader_process::CAPACITY
                .reserve(crate::shader_process::CHILD_LIMIT)
                .expect("child capacity returned"),
        );
        for name in [
            windows::core::w!("libEGL.dll"),
            windows::core::w!("libGLESv2.dll"),
        ] {
            assert!(
                unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(name) }.is_err()
            );
        }
        eprintln!("native-local-shader: backend={backend:?} shared-task=true exact-png=true exact-display=true raw-released=true parent-angle=false");
    }
}
