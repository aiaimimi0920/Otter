//! Weak-keyed compressed PNG memo cache for unchanged session rasters during checkpoints and workspace saves.
use crate::pixels::{Pixels, Storage};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock, Mutex, Weak,
    },
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_OUTOFMEMORY},
};

/// Cap compressed PNG memoization at 32 MiB and 32 live rasters; entries hold `Weak<Storage>` keys so closed or
/// edited pins are pruned immediately without pinning pixel lifetimes or consuming `DECODED_PIXEL_LIMIT` while idle.
const MAX_ENTRIES: usize = 32;
const MAX_CACHED_BYTES: usize = 32 * 1024 * 1024;

struct Entry {
    source: Weak<Storage>,
    size: (u32, u32),
    png: Arc<[u8]>,
}

static CACHE: LazyLock<Mutex<VecDeque<Entry>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(MAX_ENTRIES)));

/// Performance audit optimization (Tier 1.2):
/// Moving a single pin by 1px previously forced `encode_images` to allocate a 4K WIC bitmap, unpremultiply 8.3M
/// pixels, and run single-threaded WIC PNG zlib compression for *every* pin in the workspace (~2.7–4.0s for 15 pins).
/// Because `Pixels` storage is strictly immutable (`Weak::ptr_eq`), caching the compressed PNG bytes keyed by
/// `(Weak<Storage>, width, height)` turns unchanged-pin checkpoint encoding into an `O(1)` memory copy (`< 1 ms`).
pub(super) fn encode_cached(
    size: (u32, u32),
    pixels: &Pixels,
    cancel: &AtomicBool,
) -> Result<Pixels> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Session image encoding cancelled"));
    }
    let key = pixels.downgrade();
    if let Ok(mut guard) = CACHE.lock() {
        guard.retain(|entry| entry.source.strong_count() > 0);
        if let Some(pos) = guard
            .iter()
            .position(|entry| entry.size == size && Weak::ptr_eq(&entry.source, &key))
        {
            let hit = guard.remove(pos).unwrap();
            let png = hit.png.clone();
            guard.push_back(hit);
            drop(guard);
            return budgeted_copy(&png).ok_or_else(|| {
                Error::new(
                    E_OUTOFMEMORY,
                    "Session PNG cache copy exceeded pixel budget",
                )
            });
        }
    }
    let image = crate::desktop::image::Image {
        width: size.0,
        height: size.1,
        pixels: pixels.clone(),
    };
    let encoded = crate::desktop::export::png_checkpoint(&image, cancel)?;
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Session image encoding cancelled"));
    }
    if encoded.len() <= MAX_CACHED_BYTES / 2 {
        let shared: Arc<[u8]> = Arc::from(&encoded[..]);
        if let Ok(mut guard) = CACHE.lock() {
            guard.retain(|entry| {
                entry.source.strong_count() > 0
                    && !(entry.size == size && Weak::ptr_eq(&entry.source, &key))
            });
            let mut total: usize = guard.iter().map(|entry| entry.png.len()).sum();
            while guard.len() >= MAX_ENTRIES || total + shared.len() > MAX_CACHED_BYTES {
                let Some(evicted) = guard.pop_front() else {
                    break;
                };
                total = total.saturating_sub(evicted.png.len());
            }
            if total + shared.len() <= MAX_CACHED_BYTES {
                guard.push_back(Entry {
                    source: key,
                    size,
                    png: shared,
                });
            }
        }
    }
    Ok(encoded)
}

fn budgeted_copy(bytes: &[u8]) -> Option<Pixels> {
    let mut lease = crate::pixels::reserve(bytes.len())?;
    let mut out = Vec::new();
    out.try_reserve_exact(bytes.len()).ok()?;
    if !lease.grow_to(out.capacity()) {
        return None;
    }
    out.extend_from_slice(bytes);
    Some(Pixels::from_reserved(out, lease))
}
