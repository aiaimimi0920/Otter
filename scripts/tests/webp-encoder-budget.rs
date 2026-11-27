//! Isolated release-optimized codec admission probe. Report peak live allocator
//! bytes (including input), encoder wall time and RIFF size without file I/O.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
struct Meter;
fn add(n: usize) {
    let used = LIVE.fetch_add(n, Ordering::Relaxed) + n;
    PEAK.fetch_max(used, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            add(l.size())
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let v = System.realloc(p, l, n);
        if !v.is_null() {
            LIVE.fetch_sub(l.size(), Ordering::Relaxed);
            add(n)
        }
        v
    }
}
#[global_allocator]
static M: Meter = Meter;
fn main() {
    let mut opts = tiny_webp::Options::default();
    opts.quality = 85;
    for (w, h, mask, noise) in [
        (1920u32, 1080u32, false, false),
        (1920, 1080, true, false),
        (1920, 1080, false, true),
        (3840, 2160, true, false),
    ] {
        let mut input = vec![0; w as usize * h as usize * 4];
        let mut seed = 1u32;
        for (i, p) in input.chunks_exact_mut(4).enumerate() {
            let x = i as u32 % w;
            let y = i as u32 / w;
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let v = if noise { seed } else { (x / 7) ^ (y / 17) * 27 };
            p.copy_from_slice(&[
                v as u8,
                (v >> 3) as u8,
                (v >> 6) as u8,
                if mask && x % 50 == 0 { 128 } else { 255 },
            ]);
        }
        let baseline = LIVE.load(Ordering::Relaxed);
        PEAK.store(baseline, Ordering::Relaxed);
        let t = Instant::now();
        let bytes = tiny_webp::encode_rgba(&input, w, h, &opts).unwrap();
        let ms = t.elapsed().as_millis();
        let peak = PEAK.load(Ordering::Relaxed);
        println!("{w}x{h} alpha={mask} noise={noise} q=85 ms={ms} bytes={} peak_live_heap={peak} encoder_delta={}",bytes.len(),peak-baseline);
        assert!(
            bytes.len() < input.len()
                && peak <= 32 * w as usize * h as usize + 1_048_576
                && ms <= if w > 1920 { 8_000 } else { 3_000 }
        );
    }
}
