//! 显式消费真实旧 ShaderRenderer 浏览器回读夹具；不复制 Loom 业务算法到 Otter。
use super::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, sync::atomic::AtomicBool};

#[derive(Deserialize)]
pub(crate) struct Input {
    pub(crate) name: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}
#[derive(Deserialize)]
pub(crate) struct Case {
    pub(crate) name: String,
    pub(crate) vertex: String,
    pub(crate) fragment: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) textures: Vec<Input>,
    pub(crate) uniforms: BTreeMap<String, f32>,
    pub(crate) expected_raw_rgba: Vec<u8>,
    pub(crate) legacy_png_rgba: Vec<u8>,
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
pub(crate) fn cases() -> Vec<Case> {
    let path = std::env::var_os("OTTER_SHADER_BROWSER_CASES")
        .expect("Set OTTER_SHADER_BROWSER_CASES to the actual legacy renderer browser-cases.json");
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .unwrap()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    let cases: Vec<Case> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cases.len(), 3);
    assert_eq!(
        cases
            .iter()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        [
            "loom-identity-lut",
            "loom-exposure-alpha",
            "loom-inverted-lut-tone"
        ]
    );
    for case in &cases {
        // 绑定只读 Loom 实际合同，不接受一个自行重写的“等效滤镜”作为对照。
        assert_eq!(
            hash(&case.vertex),
            "d2c104b7898df5f13482213e67084a43309c20b9ea6be7fd41d7d539366d0c7b"
        );
        assert_eq!(
            hash(&case.fragment),
            "f0ee9d5c450a7f937e9a7967d98f8a07ff42e236826f2bcca29ebcf332f9f5cd"
        );
        assert_eq!((case.width, case.height), (3, 2));
        assert_eq!(case.expected_raw_rgba.len(), 24);
        assert_eq!(case.legacy_png_rgba.len(), 24);
    }
    // 半透明 PNG 不是 straight raw RGBA；原生显示/导出桥接必须单独处理，不能混为一谈。
    assert_eq!(&cases[0].expected_raw_rgba[12..16], &[64, 32, 16, 128]);
    assert_eq!(&cases[0].legacy_png_rgba[12..16], &[128, 64, 32, 128]);
    cases
}
#[test]
#[ignore = "requires explicit verified ANGLE DLLs and real legacy browser-cases JSON; offscreen hardware/WARP only"]
fn actual_loom_glsl_lut_matches_legacy_webgl2_raw_rgba() {
    let directory = std::env::var_os("OTTER_SHADER_ANGLE_FIXTURE")
        .map(std::path::PathBuf::from)
        .expect("Set OTTER_SHADER_ANGLE_FIXTURE to the isolated two-DLL fixture bundle");
    let cases = cases();
    std::thread::spawn(move || {
        for backend in [Backend::D3d11, Backend::Warp] {
            for case in &cases {
                let mut renderer = Renderer::new(&directory, tests::fixture_identity(), case.width, case.height, backend).unwrap();
                let textures = case.textures.iter().map(|texture| Texture {
                    name: &texture.name, width: texture.width, height: texture.height, rgba: &texture.rgba,
                }).collect::<Vec<_>>();
                let uniforms = case.uniforms.iter().map(|(name, value)| Uniform { name, value: *value }).collect::<Vec<_>>();
                let frame = renderer.render(Request { vertex: &case.vertex, fragment: &case.fragment,
                    textures: &textures, uniforms: &uniforms }, &AtomicBool::new(false)).unwrap();
                let mut maximum = 0;
                for (index, (actual, expected)) in frame.rgba.iter().zip(&case.expected_raw_rgba).enumerate() {
                    let delta = actual.abs_diff(*expected);
                    // 只允许复杂浮点 tone 的跨 GPU RGB 一字节舍入，方向/alpha 不放宽。
                    let tolerance = u8::from(case.name == "loom-inverted-lut-tone" && index % 4 != 3);
                    assert!(delta <= tolerance, "backend={backend:?} case={} byte={index} actual={actual} expected={expected}", case.name);
                    maximum = maximum.max(delta);
                }
                eprintln!("native-shader-browser-contract: backend={backend:?} case={} raw-rgba=true alpha-exact=true max-byte-delta={maximum}", case.name);
            }
        }
    }).join().unwrap();
}
