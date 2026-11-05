//! 模型门与显式 ANGLE 平台门分离；不从用户环境自动寻找 DLL，不启动浏览器或真实 Art。
use super::*;
use std::sync::atomic::AtomicBool;
pub(crate) const VERTEX: &str = "#version 300 es\nin vec2 a_position; out vec2 uv; void main(){uv=(a_position+1.0)*0.5;uv.y=1.0-uv.y;gl_Position=vec4(a_position,0,1);}";
pub(crate) const FRAGMENT: &str = "#version 300 es\nprecision highp float;uniform sampler2D u_input;uniform float u_gain;in vec2 uv;out vec4 color;void main(){vec4 p=texture(u_input,uv);color=vec4(p.rgb*u_gain,p.a);}";
fn fingerprint(bytes: u64, hex: &str) -> Fingerprint {
    let mut sha256 = [0; 32];
    for (index, byte) in sha256.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
    }
    Fingerprint { bytes, sha256 }
}
pub(crate) fn fixture_identity() -> Identity {
    Identity {
        egl: fingerprint(
            493568,
            "c92c1b5da39ef11e621f44e3cc2e53733baecdd91ae6dec42ae29f353978f4d5",
        ),
        gles: fingerprint(
            7851008,
            "915a9a4b4c826433805536c6991a4a2d427aada2a045bb932d8254445805c977",
        ),
    }
}
#[test]
fn invalid_dimensions_fail_before_library_loading() {
    for (width, height) in [(0, 1), (1, 0), (u32::MAX, 1), (16384, 16384)] {
        assert!(matches!(
            Renderer::new(
                Path::new("."),
                fixture_identity(),
                width,
                height,
                Backend::D3d11
            ),
            Err(Error::Layout)
        ));
    }
}
#[test]
fn binding_names_are_bounded_identifiers_not_source_or_paths() {
    for valid in ["input", "_lut", "gain2"] {
        assert_eq!(
            name(valid).unwrap().to_bytes(),
            format!("u_{valid}").as_bytes()
        );
    }
    for invalid in ["", "2x", "input\0x", "../lut", "a;uniform", "中文"] {
        assert!(name(invalid).is_err());
    }
    assert!(name(&"x".repeat(129)).is_err());
}
#[test]
fn empty_library_fingerprint_never_loads_a_module() {
    let empty = Identity {
        egl: Fingerprint {
            bytes: 0,
            sha256: [0; 32],
        },
        gles: fixture_identity().gles,
    };
    assert!(matches!(
        Renderer::new(Path::new("."), empty, 2, 2, Backend::D3d11),
        Err(Error::Library)
    ));
}
#[test]
fn context_lease_rejects_nested_owner_and_releases_after_failure() {
    let lease = ContextLease::acquire().unwrap();
    assert!(matches!(ContextLease::acquire(), Err(Error::ContextBusy)));
    drop(lease);
    assert!(ContextLease::acquire().is_ok());
    assert!(Renderer::new(Path::new("."), fixture_identity(), 2, 2, Backend::D3d11).is_err());
    assert!(ContextLease::acquire().is_ok());
}
#[test]
#[ignore = "requires explicit verified ANGLE fixture directory; offscreen owned D3D11/WARP only, no browser or UI"]
fn actual_verified_angle_raw_glsl_pixels_uniforms_failures_and_recreation() {
    let directory = std::env::var_os("OTTER_SHADER_ANGLE_FIXTURE")
        .map(std::path::PathBuf::from)
        .expect("Set OTTER_SHADER_ANGLE_FIXTURE to the isolated two-DLL fixture bundle");
    std::thread::spawn(move || {
        let mut wrong = fixture_identity(); wrong.egl.sha256[0] ^= 1;
        assert!(matches!(Renderer::new(&directory, wrong, 2, 2, Backend::D3d11), Err(Error::Library)));
        for backend in [Backend::D3d11, Backend::Warp] {
            for recreation in 0..3 {
                let mut renderer = Renderer::new(&directory, fixture_identity(), 2, 2, backend).unwrap();
                assert!(matches!(Renderer::new(&directory, fixture_identity(), 2, 2, backend), Err(Error::ContextBusy)));
                let other_directory = directory.clone();
                assert!(std::thread::spawn(move || matches!(Renderer::new(&other_directory, fixture_identity(), 2, 2, backend), Err(Error::ContextBusy))).join().unwrap());
                let description = renderer.description().unwrap();
                assert!(description.version.starts_with("OpenGL ES 3.0"));
                assert!(description.renderer.contains("Direct3D11"));
                // 持有已核验文件的读共享句柄，验证加载期间不能重新打开写权限。
                assert!(std::fs::OpenOptions::new().write(true).open(directory.join("libEGL.dll")).is_err());
                let pixels = [255,0,0,255,0,255,0,255,0,0,255,255,64,32,16,128];
                let textures = [Texture { name: "input", width: 2, height: 2, rgba: &pixels }];
                let uniforms = [Uniform { name: "gain", value: 1.0 }];
                let request = || Request { vertex: VERTEX, fragment: FRAGMENT, textures: &textures, uniforms: &uniforms };
                let cancel = AtomicBool::new(false);
                for _ in 0..8 {
                    let frame = renderer.render(request(), &cancel).unwrap();
                    assert_eq!((frame.width, frame.height), (2,2));
                    assert_eq!(&*frame.rgba, &pixels);
                }
                assert!(matches!(renderer.render(request(), &AtomicBool::new(true)), Err(Error::Cancelled)));
                assert!(matches!(renderer.render(Request { fragment: "#version 300 es\nnot a shader", ..request() }, &cancel), Err(Error::Compile(_))));
                let nonfinite = [Uniform { name: "gain", value: f32::NAN }];
                assert!(matches!(renderer.render(Request { uniforms: &nonfinite, ..request() }, &cancel), Err(Error::Layout)));
                let bright = [Uniform { name: "gain", value: 2.0 }];
                let frame = renderer.render(Request { uniforms: &bright, ..request() }, &cancel).unwrap();
                assert_eq!(&frame.rgba[12..], &[128,64,32,128]);
                assert_eq!(&*renderer.render(request(), &cancel).unwrap().rgba, &pixels);
                let wrong_type = FRAGMENT.replace("uniform float u_gain;", "uniform int u_gain;").replace("p.rgb*u_gain", "p.rgb*float(u_gain)");
                assert!(matches!(renderer.render(Request { fragment: &wrong_type, ..request() }, &cancel), Err(Error::Api(0x0502))));
                assert!(matches!(renderer.render(request(), &cancel), Err(Error::Poisoned)));
                eprintln!("native-shader-angle: backend={backend:?} recreation={recreation} raw-glsl=true rgba-alpha=true uniforms=true invalid-rejected=true cancel=true renderer={}", description.renderer);
            }
        }
    }).join().unwrap();
}
