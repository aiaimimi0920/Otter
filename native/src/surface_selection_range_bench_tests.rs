//! 显式微基准只测选区范围投影，不把它当作 D2D/GPU 或整个应用性能。
use super::*;
fn fixture(count: usize) -> Presentation {
    let children: Vec<_> = (0..count)
        .map(|i| {
            serde_json::json!({
                "id":format!("n{i}"),"type":"text","props":{"text":"X","selectable":true},
                "layout":{"position":"absolute","left":i%32*20,"top":i/32*20,"width":20,"height":20}
            })
        })
        .collect();
    let scene = serde_json::json!({"id":"root","type":"stack","layout":{"position":"relative","width":640,"height":640},"children":children});
    let mut p = Presentation::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        Style {
            size: 12.,
            ..style()
        },
        context(),
        [640., 640.],
    )
    .unwrap();
    let a = cross_node::point(&p, "n0", false);
    let b = cross_node::point(&p, &format!("n{}", count - 1), true);
    p.begin_text_selection(a).unwrap();
    p.finish_text_selection(b).unwrap();
    p
}
#[test]
#[ignore = "explicit range-projection microbenchmark; not a GUI performance gate"]
fn selection_range_projection_benchmark() {
    for count in [64, 256, 1024] {
        let p = fixture(count);
        let s = p.selection.as_ref().unwrap();
        for mode in ["legacy", "projected"] {
            let mut samples = Vec::new();
            for sample in 0..6 {
                let begin = std::time::Instant::now();
                let mut checksum = 0u64;
                for _ in 0..20 {
                    if mode == "projected" {
                        let span = std::hint::black_box(s).project(&p.text).unwrap();
                        for (index, item) in p.text.items().iter().enumerate() {
                            let range = std::hint::black_box(span).item_range(index, item).unwrap();
                            checksum += u64::from(range.end - range.start);
                        }
                    } else {
                        for item in p.text.items() {
                            let range = std::hint::black_box(s).item_range(item, &p.text).unwrap();
                            checksum += u64::from(range.end - range.start);
                        }
                    }
                }
                assert_eq!(checksum, count as u64 * 20);
                if sample > 0 {
                    samples.push(begin.elapsed().as_nanos());
                }
            }
            samples.sort_unstable();
            println!(
            "range_projection mode={mode} nodes={count} iterations=20 median_ns={} samples_ns={samples:?}",
            samples[2]
        );
        }
    }
}
