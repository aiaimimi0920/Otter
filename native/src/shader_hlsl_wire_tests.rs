//! 新旧协议不混用，短帧/超限/坏像素/诊断不能发布或先分配大图。
use super::*;
use crate::shader_hlsl::contract_tests::with_request;
#[test]
fn request_round_trip_and_bounded_headers_reject_mutations() {
    with_request("float4 otter_main(float2 uv){return 1;}", |r| {
        let mut encoded = Vec::new();
        write_request(&mut encoded, r, Backend::D3d12).unwrap();
        let owned = read_request(&mut encoded.as_slice()).unwrap();
        assert_eq!(owned.backend, Backend::D3d12);
        owned.with_request(|next| {
            assert_eq!(next.source, r.source);
            assert_eq!(next.textures[0].rgba, r.textures[0].rgba);
        });
        for offset in [4, 8, 12, 16, 20, 24, 28] {
            let mut bad = encoded.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(
                read_request(&mut bad.as_slice()).is_err(),
                "offset={offset}"
            );
        }
        for length in [0, 4, 31, encoded.len() - 1] {
            assert!(read_request(&mut &encoded[..length]).is_err());
        }
        let mut old = encoded.clone();
        old[..4].copy_from_slice(b"OSQ1");
        assert!(read_request(&mut old.as_slice()).is_err());
    });
}
#[test]
fn result_round_trips_typed_errors_backend_and_rejects_bad_success() {
    for error in [
        Error::Layout,
        Error::Budget,
        Error::Unsupported,
        Error::Isolation,
        Error::Cancelled,
        Error::TimedOut,
        Error::Compile("synthetic: syntax error\n".into()),
        Error::Api {
            stage: Stage::Readback,
            code: -1,
        },
    ] {
        let mut bytes = Vec::new();
        write_response(&mut bytes, &Err(error)).unwrap();
        assert!(read_response(&mut bytes.as_slice(), [1, 1])
            .unwrap()
            .is_err());
    }
    let frame = Frame {
        width: 1,
        height: 1,
        rgba: vec![255, 32, 16, 0].into(),
        backend: SelectedBackend::Warp,
    };
    let mut bytes = Vec::new();
    write_response(&mut bytes, &Ok(frame)).unwrap();
    let frame = read_response(&mut bytes.as_slice(), [1, 1])
        .unwrap()
        .unwrap();
    assert_eq!(&*frame.rgba, &[255, 32, 16, 0]);
    assert_eq!(frame.backend, SelectedBackend::Warp);
    for offset in [4, 8, 12, 16, 20, 24, 28] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(
            read_response(&mut bad.as_slice(), [1, 1]).is_err(),
            "offset={offset}"
        );
    }
    assert!(read_response(&mut &bytes[..bytes.len() - 1], [1, 1]).is_err());
    let mut malicious = Vec::new();
    write_response(&mut malicious, &Err(Error::Compile("\u{1b}[31m".into()))).unwrap();
    assert!(read_response(&mut malicious.as_slice(), [1, 1]).is_err());
}
