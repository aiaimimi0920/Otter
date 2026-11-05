//! 仅测试入口：在生产 Job/IPC 中测量滤镜后端，不改变生产 XML 准入。
use super::*;
use crate::{live_recovery_pipe::Pipe, surface_image_worker_process as process};
use std::{
    cell::Cell,
    io::{Read, Write},
    os::windows::io::AsRawHandle,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::HANDLE, System::Threading::GetExitCodeProcess};

fn fixture(mode: u8) -> (u32, String, bool) {
    let (size, primitive, repeat, reject) = match mode {
        0 => (16, "<feFlood flood-color='red'/>".into(), 1, false),
        1 => (2048, "<feTurbulence baseFrequency='.1' numOctaves='9'/>".into(), 1, true),
        2 => (2048, format!("<feConvolveMatrix order='9' kernelMatrix='{}' targetX='8' targetY='8' edgeMode='wrap'/>", "1 ".repeat(81)), 1, true),
        3 => (1024, "<feFlood flood-color='red'/>".into(), 64, true),
        4 => (2048, "<feGaussianBlur stdDeviation='100'/>".into(), 1, true),
        5 => (2048, format!("<feMerge>{}</feMerge>", "<feMergeNode in='SourceGraphic'/>".repeat(64)), 1, true),
        6 => (512, "<feTurbulence baseFrequency='.1' numOctaves='1'/>".into(), 1, false),
        7 => (1024, "<feMorphology operator='dilate' radius='4096'/>".into(), 1, false),
        8 => (1024, "<feComponentTransfer><feFuncR type='gamma' amplitude='1' exponent='2' offset='0'/></feComponentTransfer>".into(), 1, false),
        9 => (1024, "<feGaussianBlur stdDeviation='3'/>".into(), 1, false),
        _ => panic!("unknown resource fixture"),
    };
    let rect = format!("<rect width='{size}' height='{size}' fill='red' filter='url(#f)'/>");
    (size, format!("<svg xmlns='http://www.w3.org/2000/svg' width='{size}' height='{size}'><defs><filter id='f' x='0' y='0' width='100%' height='100%'>{primitive}</filter></defs>{}</svg>", rect.repeat(repeat)), reject)
}

#[test]
#[ignore = "explicit serial release resource probe with actual worker Jobs"]
fn surface_svg_work_budget_under_real_worker_job() {
    for mode in 0..=9 {
        probe(mode);
        // 每个高成本/拒绝请求后走一次新的正常 child，证明容量与管道回收。
        probe(0);
    }
}

fn probe(mode: u8) {
    let started = Instant::now();
    let deadline = started + Duration::from_secs(15);
    let (mut pipe, endpoint) = Pipe::pair().unwrap();
    let child = process::start(
        endpoint.into_handle(),
        deadline,
        &|| false,
        " --exact surface_svg_worker::work_resources::svg_work_resource_peer --ignored --nocapture",
    )
    .unwrap();
    let rejected = fixture(mode).2;
    {
        let mut io = pipe.cancellable_io(deadline, child.handle(), &|| false);
        io.write_all(crate::surface_image_worker_wire::Codec::Svg.request())
            .unwrap();
        io.write_all(&1u32.to_le_bytes()).unwrap();
        io.write_all(&[mode]).unwrap();
        let mut header = [0u8; 16];
        let result = io.read_exact(&mut header);
        if rejected {
            assert!(result.is_err(), "mode={mode}: partial success frame");
            assert_eq!(header, [0; 16], "mode={mode}: partial header");
        } else {
            result.unwrap();
            let (width, height, count) = crate::surface_image_worker_wire::response_for(
                crate::surface_image_worker_wire::Codec::Svg,
                &header,
            )
            .unwrap();
            assert_eq!((width, height, count), (1, 1, 4));
            let mut payload = [0u8; 12];
            io.read_exact(&mut payload).unwrap();
            assert_eq!(&payload[8..], &[0, 0, 255, 255]);
        }
    }
    let finish = child.finish(deadline, &|| false);
    assert_eq!(finish.is_err(), rejected, "mode={mode}: {finish:?}");
    let mut code = 0;
    unsafe { GetExitCodeProcess(HANDLE(child.handle().as_raw_handle()), &mut code) }.unwrap();
    // 42 只来自预算耗尽；区别于其他渲染错误、Job 杀进程、超时或 IPC 故障。
    assert_eq!(code, if rejected { 42 } else { 0 }, "mode={mode}");
    let peak = child.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= 128 * 1024 * 1024);
    eprintln!(
        "svg_work mode={mode} rejected={rejected} elapsed_ms={} peak_job_bytes={peak}",
        started.elapsed().as_millis()
    );
    drop(child);
}

#[test]
#[ignore = "private diagnostic decoder; never production SVG admission"]
fn svg_work_resource_peer() {
    let rejected = Cell::new(false);
    let result = host::run_for(crate::surface_image_worker_wire::Codec::Svg, |bytes| {
        let &[mode] = bytes else {
            return Err(invalid());
        };
        if mode > 9 {
            return Err(invalid());
        }
        let (size, source, _) = fixture(mode);
        let tree = resvg::usvg::Tree::from_str(&source, &resvg::usvg::Options::default())
            .map_err(|_| invalid())?;
        verify_primitive(&tree, mode);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or_else(invalid)?;
        if let Err(error) = rasterize(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        ) {
            rejected.set(error == resvg::RenderError::FilterBudgetExceeded);
            return Err(invalid());
        }
        // 测量栅格化而非传输大图片；成功才返回固定确认帧，不冒充像素等价测试。
        Ok(host::Decoded {
            logical_size: [1., 1.],
            width: 1,
            height: 1,
            pixels: vec![0, 0, 255, 255],
        })
    });
    std::process::exit(if result.is_ok() {
        0
    } else if rejected.get() {
        42
    } else {
        41
    });
}

fn verify_primitive(tree: &resvg::usvg::Tree, mode: u8) {
    use resvg::usvg::filter::Kind;
    assert!(!tree.filters().is_empty(), "fixture lost its filter");
    for filter in tree.filters() {
        assert_eq!(filter.primitives().len(), 1);
        assert!(
            matches!(
                (mode, filter.primitives()[0].kind()),
                (0 | 3, Kind::Flood(_))
                    | (1 | 6, Kind::Turbulence(_))
                    | (2, Kind::ConvolveMatrix(_))
                    | (4 | 9, Kind::GaussianBlur(_))
                    | (5, Kind::Merge(_))
                    | (7, Kind::Morphology(_))
                    | (8, Kind::ComponentTransfer(_))
            ),
            "fixture lost its expected primitive"
        );
    }
}
