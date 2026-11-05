//! 独立颜色常量oracle与真实系统编码→既有decoder；显式运行门不计入普通单测通过。
use super::*;
#[test]
fn unsupported_size_budget_and_capacity_fail_without_leaking() {
    let budget = Budget::new(0);
    for size in [
        [0, 2],
        [47, 64],
        [65, 64],
        [4096, 4096],
        [u32::MAX, 64],
        [64, 64],
    ] {
        assert!(VideoEncoder::create(size, &budget).is_err());
        assert_eq!(budget.used(), 0);
    }
    let permits: Vec<_> = (0..4).map(|_| Permit::acquire().unwrap()).collect();
    assert!(Permit::acquire().is_err());
    drop(permits);
    drop(Permit::acquire().unwrap());
}
#[test]
#[ignore = "requires Windows system H264 encoder and decoder; synthetic pixels only"]
fn system_encoder_roundtrip_idr_delta_color_and_owned_output_budget() {
    let stop = AtomicBool::new(false);
    for size in [[64, 64], [680, 430], [1920, 1080]] {
        let budget = Budget::new(64 * 1024 * 1024);
        let mut encoder = VideoEncoder::create(size, &budget).unwrap();
        let baseline = budget.used();
        let mut decoder = super::super::DecoderOwner::create()
            .unwrap()
            .configure(size[0], size[1])
            .unwrap()
            .into_video();
        let bgra = [0, 0, 255, 255].repeat(size[0] as usize * size[1] as usize);
        let mut retained = None;
        for id in 0..4 {
            let encoded = encoder.encode(&bgra, id * 333333, id == 2, &stop).unwrap();
            assert_eq!(encoded.keyframe, id % 2 == 0);
            assert_eq!(budget.used(), baseline + MAX_AU);
            let decoded = decoder
                .decode(&encoded.bytes, id * 333333, encoded.keyframe, &stop)
                .unwrap()
                .unwrap();
            assert_eq!(decoded.timestamp, id * 333333);
            for p in decoded.pixels.chunks_exact(4) {
                assert!(p[2] > 220 && p[0] < 25 && p[1] < 25 && p[3] == 255, "{p:?}");
            }
            if id == 3 {
                retained = Some(encoded);
            }
        }
        drop(encoder);
        assert_eq!(budget.used(), MAX_AU);
        assert!(!retained.as_ref().unwrap().bytes.is_empty());
        drop(retained);
        assert_eq!(budget.used(), 0);
        eprintln!("native-source-encoder: size={size:?} idr=0,2 delta=1,3 decoded-red=true lease-released=true");
    }
}
#[test]
#[ignore = "requires Windows system encoder; verifies immediate retirement on errors"]
fn system_encoder_alpha_cancel_replay_and_output_budget_retire_owner() {
    for mode in 0..4 {
        let budget = Budget::new(8 * 1024 * 1024);
        let mut encoder = VideoEncoder::create([64, 64], &budget).unwrap();
        let mut bgra = [0, 255, 0, 255].repeat(64 * 64);
        let mut hold = None;
        let stop = AtomicBool::new(mode == 1);
        if mode == 0 {
            bgra[3] = 128;
        }
        if mode == 2 {
            drop(encoder.encode(&bgra, 1, false, &stop).unwrap());
        }
        if mode == 3 {
            hold = budget.reserve(8 * 1024 * 1024 - budget.used());
        }
        assert!(encoder
            .encode(&bgra, if mode == 2 { 1 } else { 0 }, false, &stop)
            .is_err());
        assert!(encoder.configured.is_none());
        assert_eq!(budget.used(), hold.as_ref().map_or(0, Lease::bytes));
        drop(hold);
        assert_eq!(budget.used(), 0);
        assert!(encoder
            .encode(&bgra, 2, true, &AtomicBool::new(false))
            .is_err());
    }
}
