//! Experimental element-owned length adjustment over actual shaped clusters.
use super::{GlyphCluster, Text, TextAnchor, TextChunk};
use crate::LengthAdjust;

#[path = "length_fragments.rs"]
mod length_fragments;

pub(super) fn apply(text: &Text, chunks: &mut [Vec<GlyphCluster>]) -> Option<()> {
    let groups = length_fragments::groups(text, chunks);
    // Direct children are disjoint source ranges. Index once instead of searching
    // every constraint for every glyph, even when visual order is bidi-reordered.
    let mut children = vec![Vec::new(); text.length_ranges.len()];
    for (index, range) in text.length_ranges.iter().enumerate() {
        if let Some(parent) = range.parent() {
            children[parent].push(index);
        }
    }
    let mut locations = Vec::new();
    let mut bytes = Vec::new();
    let mut positions = Vec::new();
    let mut advances = Vec::new();
    let (mut byte_base, mut char_base, mut pen) = (0, 0, 0.0);
    for (chunk_index, (chunk, clusters)) in text.chunks.iter().zip(chunks.iter()).enumerate() {
        for (index, cluster) in clusters.iter().enumerate() {
            let cp = char_base + cluster.byte_idx.code_point_at(&chunk.text);
            pen += f64::from(text.dx.get(cp).copied().unwrap_or(0.0));
            locations.push((chunk_index, index));
            bytes.push(byte_base + cluster.byte_idx.value());
            positions.push(pen);
            advances.push(f64::from(cluster.advance));
            pen += f64::from(cluster.advance);
        }
        byte_base += chunk.text.len();
        char_base += chunk.text.chars().count();
    }
    let original_positions = positions.clone();
    let original_advances = advances.clone();
    let mut scales = vec![1.0; positions.len()];
    let mut resolved = vec![false; positions.len()];
    // Source order and visual order differ for bidi text. Query a source index,
    // then restore visual order before applying the existing adjustment rules.
    let mut source_order: Vec<_> = bytes.iter().copied().enumerate().collect();
    source_order.sort_unstable_by_key(|&(index, byte)| (byte, index));
    for (id, range) in text.length_ranges.iter().enumerate().rev() {
        let bounds = range.range();
        let start = source_order.partition_point(|&(_, byte)| byte < bounds.start);
        let end = source_order.partition_point(|&(_, byte)| byte < bounds.end);
        let mut selected: Vec<_> = source_order[start..end].iter().map(|&(i, _)| i).collect();
        selected.sort_unstable();
        if selected.is_empty() {
            continue;
        }
        let min = selected
            .iter()
            .map(|&i| positions[i].min(positions[i] + advances[i]))
            .fold(f64::INFINITY, f64::min);
        let max = selected
            .iter()
            .map(|&i| positions[i].max(positions[i] + advances[i]))
            .fold(f64::NEG_INFINITY, f64::max);
        let delta = f64::from(range.length()) - (max - min);
        let following_shift = if range.adjustment() == LengthAdjust::Spacing {
            let mut seen = std::collections::HashSet::new();
            let adjustable: Vec<_> = selected
                .iter()
                .map(|&index| {
                    let siblings = &children[id];
                    let candidate = siblings.partition_point(|&child| {
                        text.length_ranges[child].range().end <= bytes[index]
                    });
                    let child = siblings
                        .get(candidate)
                        .copied()
                        .filter(|&child| text.length_ranges[child].range().contains(&bytes[index]));
                    child.is_none_or(|child| seen.insert(child))
                })
                .collect();
            let count = adjustable.iter().filter(|flag| **flag).count();
            // A single typographic unit has no adjustable gap, but its range is
            // still resolved and must be protected from ancestor glyph scaling.
            let step = if count < 2 {
                0.0
            } else {
                delta / (count - 1) as f64
            };
            let mut shift = 0.0;
            for (&index, adjustable) in selected.iter().zip(adjustable) {
                positions[index] += shift;
                if adjustable {
                    shift += step;
                }
            }
            shift
        } else {
            if max <= min {
                continue;
            }
            let scale = f64::from(range.length()) / (max - min);
            if !scale.is_finite() || scale <= 0.0 {
                continue;
            }
            let mut anchors = std::collections::HashMap::new();
            for &index in &selected {
                let position = positions[index];
                let adjusted = min + (position - min) * scale;
                positions[index] = if resolved[index] {
                    let (old, new) = *anchors.entry(groups[index]).or_insert((position, adjusted));
                    new + position - old
                } else {
                    adjusted
                };
                // Ancestors move resolved descendants without stretching their
                // glyphs again. Position grouping needs separate shaping data.
                if !resolved[index] {
                    advances[index] *= scale;
                    scales[index] *= scale;
                }
            }
            delta
        };
        for &index in &selected {
            resolved[index] = true;
        }
        if let Some(&last) = selected.last() {
            for position in &mut positions[last + 1..] {
                *position += following_shift;
            }
        }
    }
    let mut previous_delta = 0.0;
    let mut previous_advance_delta = 0.0;
    for (i, &(chunk_index, index)) in locations.iter().enumerate() {
        let delta = positions[i] - original_positions[i];
        let cluster = &mut chunks[chunk_index][index];
        cluster.length_shift = if index == 0 && text.chunks[chunk_index].x.is_some() {
            0.0
        } else {
            (delta - previous_delta - previous_advance_delta) as f32
        };
        cluster.length_scale = scales[i] as f32;
        cluster.width *= scales[i] as f32;
        cluster.advance = advances[i] as f32;
        if !cluster.length_shift.is_finite()
            || !cluster.length_scale.is_finite()
            || !cluster.width.is_finite()
            || !cluster.advance.is_finite()
        {
            return None;
        }
        previous_delta = delta;
        previous_advance_delta = advances[i] - original_advances[i];
    }
    Some(())
}

pub(super) fn anchor(
    text: &Text,
    chunk: &TextChunk,
    offset: usize,
    clusters: &[GlyphCluster],
) -> f32 {
    let (mut pen, mut first, mut min, mut max) = (0.0_f64, None, f64::INFINITY, f64::NEG_INFINITY);
    for cluster in clusters {
        let cp = offset + cluster.byte_idx.code_point_at(&chunk.text);
        pen += f64::from(text.dx.get(cp).copied().unwrap_or(0.0)) + f64::from(cluster.length_shift);
        first.get_or_insert(pen);
        min = min.min(pen).min(pen + f64::from(cluster.advance));
        max = max.max(pen).max(pen + f64::from(cluster.advance));
        pen += f64::from(cluster.advance);
    }
    first
        .map(|first| {
            (first
                - match chunk.anchor {
                    TextAnchor::Start => min,
                    TextAnchor::Middle => (min + max) / 2.0,
                    TextAnchor::End => max,
                }) as f32
        })
        .unwrap_or(0.0)
}
