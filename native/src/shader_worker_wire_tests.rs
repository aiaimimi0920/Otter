//! 纯线协议门：伪造长度/状态/绑定在驱动加载前拒绝，输出明确保持 raw RGBA。
use super::*;
use crate::{
    shader_angle::{self as angle, Fingerprint, Identity, Texture, Uniform},
    shader_process::TrustedBundle,
};
use std::io::Cursor;
pub(crate) fn bundle() -> TrustedBundle {
    TrustedBundle::new(
        std::path::Path::new("."),
        Identity {
            egl: Fingerprint {
                bytes: 1,
                sha256: [0; 32],
            },
            gles: Fingerprint {
                bytes: 1,
                sha256: [0; 32],
            },
        },
    )
    .unwrap()
}
fn encoded() -> Vec<u8> {
    let bundle = bundle();
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 64, 32, 16, 128,
    ];
    let textures = [Texture {
        name: "input",
        width: 2,
        height: 2,
        rgba: &pixels,
    }];
    let uniforms = [Uniform {
        name: "gain",
        value: 1.0,
    }];
    let invocation = Invocation {
        bundle: &bundle,
        backend: Backend::Warp,
        width: 2,
        height: 2,
        request: Request {
            vertex: "vertex",
            fragment: "fragment",
            textures: &textures,
            uniforms: &uniforms,
        },
    };
    let mut bytes = Vec::new();
    write_request(
        &mut bytes,
        &invocation,
        &Header::for_invocation(&invocation).unwrap(),
    )
    .unwrap();
    bytes
}
#[test]
fn shader_wire_roundtrip_preserves_raw_payload_and_trusted_bundle() {
    let bytes = encoded();
    let owned = read_request(&mut Cursor::new(&bytes)).unwrap();
    owned.with_invocation(|value| {
        assert_eq!((value.width, value.height), (2, 2));
        assert_eq!(value.request.uniforms[0].value, 1.0);
        assert_eq!(&value.request.textures[0].rgba[12..], &[64, 32, 16, 128]);
        let mut again = Vec::new();
        write_request(&mut again, &value, &Header::for_invocation(&value).unwrap()).unwrap();
        assert_eq!(again, bytes);
    });
}
#[test]
fn shader_request_rejects_headers_truncation_payload_sizes_names_and_nan() {
    let good = encoded();
    for (offset, value) in [
        (0, 0),
        (4, u32::MAX),
        (8, 2),
        (12, 0),
        (16, 8193),
        (20, u32::MAX),
        (24, 0),
        (28, MAX_SOURCE as u32 + 1),
        (32, 33),
        (36, 257),
    ] {
        let mut bad = good.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(
            read_request(&mut Cursor::new(bad)).is_err(),
            "header offset={offset}"
        );
    }
    for length in [0, 1, 39, 40, good.len() / 2, good.len() - 1] {
        assert!(read_request(&mut Cursor::new(&good[..length])).is_err());
    }
    let header = Header::read(&mut Cursor::new(&good)).unwrap();
    let vertex = 40 + 80 + header.path_units as usize * 2;
    let texture = vertex + header.vertex as usize + header.fragment as usize;
    for offset in [vertex, texture + 16] {
        let mut bad = good.clone();
        bad[offset] = 0;
        assert!(read_request(&mut Cursor::new(bad)).is_err());
    }
    let mut bad = good.clone();
    bad[texture + 12..texture + 16].copy_from_slice(&1u32.to_le_bytes());
    assert!(read_request(&mut Cursor::new(bad)).is_err());
    let mut bad = good.clone();
    bad[texture + 16..texture + 21].copy_from_slice(b"other");
    assert!(read_request(&mut Cursor::new(bad)).is_err());
    let mut bad = good.clone();
    let value = bad.len() - 8;
    bad[value..value + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(read_request(&mut Cursor::new(bad)).is_err());
    let mut bad = good;
    bad[4..8].copy_from_slice(&(header.body + 1).to_le_bytes());
    bad.push(0);
    assert!(read_request(&mut Cursor::new(bad)).is_err());
}
#[test]
fn shader_short_body_rejects_large_texture_before_allocation() {
    let mut bytes = encoded();
    let header = Header::read(&mut Cursor::new(&bytes)).unwrap();
    let texture = 40
        + 80
        + header.path_units as usize * 2
        + header.vertex as usize
        + header.fragment as usize;
    for (offset, value) in [(4, 8192u32), (8, 4096), (12, 128 * 1024 * 1024)] {
        bytes[texture + offset..texture + offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let error = read_request(&mut Cursor::new(bytes)).err().unwrap();
    // 旧路径会先分配 128 MiB，随后才得到 UnexpectedEof。
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);

    let mut bytes = encoded();
    bytes.truncate(bytes.len() - 12);
    let body = (bytes.len() - 40) as u32;
    bytes[4..8].copy_from_slice(&body.to_le_bytes());
    assert_eq!(
        read_request(&mut Cursor::new(bytes)).err().unwrap().kind(),
        io::ErrorKind::InvalidData
    );
}
#[test]
fn shader_preflight_checks_duplicate_bindings_and_old_4k_policy() {
    let pixels = [0; 4];
    let mut textures = [
        Texture {
            name: "input",
            width: 1,
            height: 1,
            rgba: &pixels,
        },
        Texture {
            name: "input",
            width: 1,
            height: 1,
            rgba: &pixels,
        },
    ];
    {
        let request = Request {
            vertex: "v",
            fragment: "f",
            textures: &textures,
            uniforms: &[],
        };
        assert!(validate(1, 1, &request).is_err());
    }
    textures[1].name = "lut";
    let mut request = Request {
        vertex: "v",
        fragment: "f",
        textures: &textures,
        uniforms: &[],
    };
    assert!(validate(1, 1, &request).is_ok());
    let uniforms = [Uniform {
        name: "lut",
        value: 1.0,
    }];
    request.uniforms = &uniforms;
    assert!(validate(1, 1, &request).is_err());
    assert_eq!(pixel_count(3840, 2160).unwrap(), 33_177_600);
    for size in [(0, 1), (8193, 1), (8192, 8192), (u32::MAX, u32::MAX)] {
        assert!(pixel_count(size.0, size.1).is_err());
    }
}
#[test]
fn shader_response_roundtrip_keeps_rgba_and_typed_bounded_diagnostics() {
    let frame = angle::Frame {
        width: 1,
        height: 1,
        rgba: vec![64, 32, 16, 128].into(),
    };
    let mut bytes = Vec::new();
    write_response(&mut bytes, &Ok(frame)).unwrap();
    assert_eq!(
        &*read_response(&mut Cursor::new(&bytes), [1, 1])
            .unwrap()
            .unwrap()
            .rgba,
        &[64, 32, 16, 128]
    );
    let failures = [
        angle::Error::Compile("中文".repeat(2000)),
        angle::Error::Link("link".into()),
        angle::Error::Layout,
        angle::Error::Budget,
        angle::Error::Library,
        angle::Error::Context,
        angle::Error::Cancelled,
        angle::Error::Api(0x0502),
        angle::Error::Poisoned,
        angle::Error::ContextBusy,
    ];
    for error in failures {
        let mut bytes = Vec::new();
        write_response(&mut bytes, &Err(error)).unwrap();
        assert!(bytes.len() <= 24 + 4096);
        assert!(read_response(&mut Cursor::new(bytes), [1, 1])
            .unwrap()
            .is_err());
    }
}
#[test]
fn shader_response_rejects_unknown_status_dimensions_count_diagnostics_and_truncation() {
    let mut good = Vec::new();
    write_response(
        &mut good,
        &Ok(angle::Frame {
            width: 1,
            height: 1,
            rgba: vec![64, 32, 16, 128].into(),
        }),
    )
    .unwrap();
    for (offset, value) in [(0, 0), (4, 11), (8, 1), (12, 2), (16, 0), (20, u32::MAX)] {
        let mut bad = good.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(read_response(&mut Cursor::new(bad), [1, 1]).is_err());
    }
    for length in [0, 1, 23, 24, 27] {
        assert!(read_response(&mut Cursor::new(&good[..length]), [1, 1]).is_err());
    }
    let mut bad = Vec::new();
    write_response(&mut bad, &Err(angle::Error::Compile("a".into()))).unwrap();
    bad[24] = 0x1b;
    assert!(read_response(&mut Cursor::new(bad), [1, 1]).is_err());
}
