//! 真实产品 child → Canvas alpha 桥接 → PNG；oracle 来自独立解码的真实浏览器矩阵。
use super::*;
use crate::desktop::shader_image::{self, tests as oracle};
use std::{io::Read, sync::atomic::AtomicBool};

fn browser_matrix() -> Vec<u8> {
    let root = std::env::var_os("OTTER_SHADER_ALPHA_FIXTURE")
        .expect("Set OTTER_SHADER_ALPHA_FIXTURE to the verified Canvas alpha fixture directory");
    let mut bytes = Vec::new();
    std::fs::File::open(PathBuf::from(root).join("alpha-straight.rgba"))
        .unwrap()
        .take(256 * 256 * 4 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes.len(), 256 * 256 * 4);
    assert_eq!(oracle::hash(&bytes), oracle::STRAIGHT_SHA256);
    bytes
}
fn expected(raw: &[u8], matrix: &[u8]) -> Vec<u8> {
    raw.chunks_exact(4)
        .flat_map(|p| {
            let channel = |c: u8| matrix[(usize::from(p[3]) * 256 + usize::from(c)) * 4];
            [channel(p[0]), channel(p[1]), channel(p[2]), p[3]]
        })
        .collect()
}
fn assert_bridge(frame: Frame, expected: &[u8]) -> usize {
    let (width, height) = (frame.width, frame.height);
    let weak = frame.rgba.downgrade();
    let output = shader_image::prepare(frame, &AtomicBool::new(false)).unwrap();
    assert!(
        weak.upgrade().is_none(),
        "input raw frame must not remain in the prepared result"
    );
    let actual = oracle::decode_rgba(&output.png, width, height);
    let difference = actual
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (a, e))| a != e)
        .map(|(at, (a, e))| (at, *a, *e));
    assert_eq!(actual.len(), expected.len());
    assert_eq!(
        difference, None,
        "straight PNG alpha/orientation must be exact"
    );
    for (straight, display) in actual
        .chunks_exact(4)
        .zip(output.image.pixels.chunks_exact(4))
    {
        let premultiply = |c: u8| ((u16::from(c) * u16::from(straight[3]) + 127) / 255) as u8;
        assert_eq!(
            display,
            &[
                premultiply(straight[2]),
                premultiply(straight[1]),
                premultiply(straight[0]),
                straight[3]
            ]
        );
    }
    let size = output.png.len();
    let png = output.png.clone();
    let batch = output.into_batch().unwrap();
    let formal = batch.primary().value.image_value().unwrap();
    assert!(
        formal.encoded_png().unwrap().same_storage(&png),
        "formal batch must retain the same verified PNG"
    );
    size
}

#[test]
#[ignore = "requires verified ANGLE, actual browser alpha/GLSL fixtures and freshly built same-profile product EXE"]
fn actual_product_shader_png_and_display_match_complete_browser_alpha_matrix() {
    let bundle = bundle();
    let matrix = browser_matrix();
    let source = oracle::matrix();
    let textures = [Texture {
        name: "input",
        width: source.width,
        height: source.height,
        rgba: &source.rgba,
    }];
    let uniforms = [Uniform {
        name: "gain",
        value: 1.0,
    }];
    for backend in [Backend::D3d11, Backend::Warp] {
        let (result, peak) = execute(
            Invocation {
                bundle: &bundle,
                backend,
                width: source.width,
                height: source.height,
                request: Request {
                    vertex: angle::tests::VERTEX,
                    fragment: angle::tests::FRAGMENT,
                    textures: &textures,
                    uniforms: &uniforms,
                },
            },
            true,
        );
        let size = assert_bridge(result.unwrap(), &matrix);
        eprintln!("native-shader-image-matrix: backend={backend:?} pixels=65536 exact-png=true exact-display=true png-bytes={size} peak-job-bytes={peak}");
    }
}

#[test]
#[ignore = "requires verified ANGLE/browser fixtures and freshly built same-profile product EXE"]
fn actual_product_loom_shader_png_and_display_match_browser_raw_plus_alpha_oracle() {
    let bundle = bundle();
    let matrix = browser_matrix();
    for backend in [Backend::D3d11, Backend::Warp] {
        for case in angle::contract_tests::cases() {
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
                true,
            );
            // 旧三组 legacy_png_rgba 是 PNG→Canvas2D 后值，不误作编码 PNG 的 straight oracle。
            let size = assert_bridge(result.unwrap(), &expected(&case.expected_raw_rgba, &matrix));
            eprintln!("native-shader-image-loom: backend={backend:?} case={} exact-png=true exact-display=true png-bytes={size} peak-job-bytes={peak}", case.name);
        }
    }
}

#[test]
#[ignore = "requires verified ANGLE/browser fixtures and freshly built same-profile product EXE; actual 4K allocation"]
fn actual_product_4k_shader_png_and_display_preserve_alpha_and_orientation() {
    let bundle = bundle();
    let matrix = browser_matrix();
    let (width, height) = (3840, 2160);
    let pixels = (0..width * height)
        .flat_map(|index| {
            let x = index % width;
            let y = index / width;
            [
                x as u8,
                y as u8,
                (x ^ y) as u8,
                [0, 64, 128, 255][index as usize % 4],
            ]
        })
        .collect::<Vec<_>>();
    let expected = expected(&pixels, &matrix);
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
        let started = Instant::now();
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
        let size = assert_bridge(result.unwrap(), &expected);
        eprintln!("native-shader-image-4k: backend={backend:?} bytes={} exact-png=true exact-display=true png-bytes={size} peak-job-bytes={peak} elapsed-ms={}", pixels.len(), started.elapsed().as_millis());
    }
}
