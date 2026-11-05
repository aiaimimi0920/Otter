//! 显式 GPU 门只运行自有离屏纹理；真实平台失败不能 ignored/0 tests 冒充通过。
use super::*;
const COPY: &str = "float4 otter_main(float2 uv){return u_input.Sample(otter_sampler,uv);}";
fn frame(
    backend: Backend,
    width: u32,
    height: u32,
    pixels: &[u8],
    source: &str,
    uniforms: &[Uniform<'_>],
    auxiliary: &[Texture<'_>],
) -> std::result::Result<Frame, process::Failure> {
    let mut textures = vec![Texture {
        name: "input",
        width,
        height,
        rgba: pixels,
    }];
    textures.extend_from_slice(auxiliary);
    let request = Request {
        abi: ABI,
        source,
        width,
        height,
        textures: &textures,
        uniforms,
    };
    if width == 3840 && height == 2160 {
        let (result, peak) = process::render_observed(&request, backend);
        eprintln!("hlsl-4k: backend={backend:?} peak-job-bytes={peak} exited=true");
        result
    } else {
        process::render(&request, backend, &|| false)
    }
}
fn actual(backend: Backend, expected: SelectedBackend) {
    let start = Instant::now();
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 127, 32, 0, 64, 32, 16, 128, 16, 64,
        128, 255,
    ];
    for _ in 0..3 {
        let result = frame(backend, 3, 2, &pixels, COPY, &[], &[]).unwrap();
        assert_eq!(result.backend, expected);
        assert_eq!(&*result.rgba, &pixels);
    }
    let clipped = frame(
        backend,
        3,
        2,
        &pixels,
        "float4 otter_main(float2 uv){clip(-1);return 1;}",
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(
        &*clipped.rgba, &[0; 24],
        "fully clipped pixels are transparent black"
    );
    let mut partial = pixels;
    partial[..4].fill(0);
    partial[12..16].fill(0);
    let discarded = frame(
        backend,
        3,
        2,
        &pixels,
        "float4 otter_main(float2 uv){if(uv.x<0.5)discard;return u_input.Sample(otter_sampler,uv);}",
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(
        &*discarded.rgba, &partial,
        "only discarded pixels clear; straight channels stay intact"
    );
    let mut matrix = Vec::with_capacity(256 * 256 * 4);
    for alpha in 0..=255u8 {
        for red in 0..=255u8 {
            matrix.extend_from_slice(&[red, 255 - red, alpha, alpha]);
        }
    }
    let result = frame(backend, 256, 256, &matrix, COPY, &[], &[]).unwrap();
    assert_eq!(
        &*result.rgba, &matrix,
        "straight alpha and no implicit sRGB"
    );
    let auxiliary_pixels = [
        0, 0, 0, 0, 128, 0, 0, 128, 0, 128, 0, 128, 128, 128, 128, 128,
    ];
    let aux = [Texture {
        name: "mask",
        width: 2,
        height: 2,
        rgba: &auxiliary_pixels,
    }];
    let sample =
        "float4 otter_main(float2 uv){return u_mask.Sample(otter_sampler,float2(u_x,u_y))*u_gain;}";
    for (x, y, gain, expected_pixel) in [
        (0.5, 0.5, 1., [64, 64, 32, 96]),
        (-4., -4., 1., [0, 0, 0, 0]),
        (4., 4., 0.5, [64, 64, 64, 64]),
    ] {
        let uniforms = [
            Uniform {
                name: "x",
                value: x,
            },
            Uniform {
                name: "y",
                value: y,
            },
            Uniform {
                name: "gain",
                value: gain,
            },
        ];
        let result = frame(backend, 1, 1, &[0; 4], sample, &uniforms, &aux).unwrap();
        for (value, expected) in result.rgba.iter().zip(expected_pixel) {
            assert!(value.abs_diff(expected) <= 1);
        }
    }
    let names = (0..256).map(|i| format!("p{i}")).collect::<Vec<_>>();
    let uniforms = names
        .iter()
        .enumerate()
        .map(|(i, name)| Uniform {
            name,
            value: if i == 255 { 0.25 } else { 0. },
        })
        .collect::<Vec<_>>();
    assert_eq!(
        &*frame(
            backend,
            1,
            1,
            &[0; 4],
            "float4 otter_main(float2 uv){return float4(u_p255,0,0,1);}",
            &uniforms,
            &[]
        )
        .unwrap()
        .rgba,
        &[64, 0, 0, 255]
    );
    let names = (0..31).map(|i| format!("image{i}")).collect::<Vec<_>>();
    let textures = names
        .iter()
        .map(|name| Texture {
            name,
            width: 1,
            height: 1,
            rgba: &[25, 50, 100, 255],
        })
        .collect::<Vec<_>>();
    assert_eq!(
        &*frame(
            backend,
            1,
            1,
            &[0; 4],
            "float4 otter_main(float2 uv){return u_image30.Sample(otter_sampler,uv);}",
            &[],
            &textures
        )
        .unwrap()
        .rgba,
        &[25, 50, 100, 255]
    );
    for source in [
        "not valid HLSL",
        "#include \"C:/Windows/win.ini\"\nfloat4 otter_main(float2 uv){return 1;}",
    ] {
        assert!(
            matches!(frame(backend, 1, 1, &[0; 4], source, &[], &[]), Err(process::Failure::Backend(Error::Compile(message))) if !message.is_empty() && message.len() <= 4096)
        );
    }
    // 同一个后端在编译失败 child 完全退出后仍可创建，不保留失败程序/纹理。
    assert!(matches!(frame(backend, 1, 1, &[0;4], "Texture2D<float4> foreign:register(t99); float4 otter_main(float2 uv){return foreign.Sample(otter_sampler,uv);}", &[], &[]), Err(process::Failure::Backend(Error::Layout))));
    assert_eq!(
        &*frame(backend, 3, 2, &pixels, COPY, &[], &[]).unwrap().rgba,
        &pixels
    );
    eprintln!("hlsl-native: backend={expected:?} recreation=3 coordinates=true discard-clear=true straight-alpha-matrix=65536 texture-slots=32 scalar-slots=256 linear-clamp=true includes-rejected=true elapsed-ms={}", start.elapsed().as_millis());
}
#[test]
#[ignore = "explicit owned offscreen D3D12 GPU gate; coordinate with MAIN"]
fn actual_d3d12_pixels_textures_parameters_compiler_and_cleanup() {
    actual(Backend::D3d12, SelectedBackend::D3d12);
}
#[test]
#[ignore = "explicit owned offscreen D3D11 GPU gate; coordinate with MAIN"]
fn actual_d3d11_pixels_textures_parameters_compiler_and_cleanup() {
    actual(Backend::D3d11, SelectedBackend::D3d11);
}
#[test]
#[ignore = "explicit owned offscreen WARP gate; not a hardware/CPU UI claim"]
fn actual_warp_pixels_textures_parameters_compiler_and_cleanup() {
    actual(Backend::Warp, SelectedBackend::Warp);
}
#[test]
#[ignore = "explicit owned 4K/Auto GPU gate; coordinate with MAIN"]
fn actual_auto_prefers_d3d12_and_4k_output_is_bounded() {
    let pixels = vec![96; 3840 * 2160 * 4];
    for (backend, expected) in [
        (Backend::Auto, SelectedBackend::D3d12),
        (Backend::D3d11, SelectedBackend::D3d11),
        (Backend::Warp, SelectedBackend::Warp),
    ] {
        let result = frame(backend, 3840, 2160, &pixels, COPY, &[], &[]).unwrap();
        assert_eq!(result.backend, expected);
        assert_eq!(&*result.rgba, &pixels);
    }
    assert!(matches!(
        frame(Backend::Software, 1, 1, &[0; 4], COPY, &[], &[]),
        Err(process::Failure::Backend(Error::Unsupported))
    ));
}
