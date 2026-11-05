//! 无 GPU 的合同门：准入、固定布局、软件拒绝与子进程专属入口。
use super::*;
pub(super) fn with_request<T>(source: &str, run: impl FnOnce(&Request<'_>) -> T) -> T {
    let pixels = [64, 32, 16, 128];
    let textures = [Texture {
        name: "input",
        width: 1,
        height: 1,
        rgba: &pixels,
    }];
    run(&Request {
        abi: ABI,
        source,
        width: 1,
        height: 1,
        textures: &textures,
        uniforms: &[],
    })
}
#[test]
fn layout_rejects_version_source_sizes_names_collisions_and_nonfinite_before_gpu() {
    with_request("float4 otter_main(float2 uv){return 1;}", |r| {
        validate(r).unwrap();
        assert!(validate(&Request {
            abi: "shader",
            ..*r
        })
        .is_err());
        for (width, height) in [(0, 1), (1, 0), (8193, 1), (8192, 8192), (u32::MAX, 1)] {
            assert!(validate(&Request {
                width,
                height,
                ..*r
            })
            .is_err());
        }
        for source in ["", "x\0y"] {
            assert!(validate(&Request { source, ..*r }).is_err());
        }
        assert!(validate(&Request {
            source: &"x".repeat(MAX_SOURCE + 1),
            ..*r
        })
        .is_err());
        for name in ["", "2x", "a;", "../x", "中文", "otter_size"] {
            assert!(contract::name(name).is_err());
        }
        for name in ["input", "gain2", "_mask"] {
            contract::name(name).unwrap();
        }
        assert!(contract::name(&"x".repeat(129)).is_err());
        let uniforms = [Uniform {
            name: "input",
            value: 0.,
        }];
        assert!(validate(&Request {
            uniforms: &uniforms,
            ..*r
        })
        .is_err());
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(validate(&Request {
                uniforms: &[Uniform {
                    name: "gain",
                    value
                }],
                ..*r
            })
            .is_err());
        }
        let duplicate = [r.textures[0], r.textures[0]];
        assert!(validate(&Request {
            textures: &duplicate,
            ..*r
        })
        .is_err());
        let uniforms = [
            Uniform {
                name: "gain",
                value: 1.,
            },
            Uniform {
                name: "gain",
                value: 2.,
            },
        ];
        assert!(validate(&Request {
            uniforms: &uniforms,
            ..*r
        })
        .is_err());
    });
}
#[test]
fn scalar_layout_is_fixed_16_byte_stride_and_names_are_not_unchecked_source() {
    with_request(
        "float4 otter_main(float2 uv){return u_input.Sample(otter_sampler,uv)*u_gain;}",
        |r| {
            let uniforms = [
                Uniform {
                    name: "gain",
                    value: 0.5,
                },
                Uniform {
                    name: "enabled",
                    value: 1.,
                },
            ];
            let r = Request {
                uniforms: &uniforms,
                ..*r
            };
            validate(&r).unwrap();
            let data = contract::constants(&r);
            assert_eq!(data.len(), 48);
            assert_eq!(&data[16..20], &0.5f32.to_le_bytes());
            assert_eq!(&data[32..36], &1f32.to_le_bytes());
            let source = compile::source(&r);
            for text in [
                "Texture2D<float4> u_input:register(t0);",
                "float u_gain:packoffset(c1.x);",
                "float u_enabled:packoffset(c2.x);",
            ] {
                assert!(source.contains(text));
            }
        },
    );
}
#[test]
fn unsupported_software_cancel_deadline_and_unisolated_execution_are_explicit() {
    with_request("float4 otter_main(float2 uv){return 1;}", |r| {
        let deadline = Instant::now() + std::time::Duration::from_secs(1);
        assert_eq!(
            render(r, Backend::Software, &|| false, deadline).unwrap_err(),
            Error::Unsupported
        );
        assert_eq!(
            render(r, Backend::Auto, &|| true, deadline).unwrap_err(),
            Error::Cancelled
        );
        assert_eq!(
            render(r, Backend::Auto, &|| false, Instant::now()).unwrap_err(),
            Error::TimedOut
        );
        assert_eq!(
            render(r, Backend::Auto, &|| false, deadline).unwrap_err(),
            Error::Isolation
        );
    });
    assert!(process::parse(&[process::FLAG.into()]).unwrap());
    assert!(!process::parse(&["--smoke-test".into()]).unwrap());
    assert!(process::parse(&[process::FLAG.into(), "--settings".into()]).is_err());
}
