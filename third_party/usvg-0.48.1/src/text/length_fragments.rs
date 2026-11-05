//! Fragment boundaries retained for nested scaling, not arbitrary source ranges.
use super::*;

pub(super) fn groups(text: &Text, chunks: &[Vec<GlyphCluster>]) -> Vec<usize> {
    let mut result = Vec::new();
    let mut base = 0;
    let mut group = 0;
    for (chunk, clusters) in text.chunks.iter().zip(chunks) {
        let mut previous = None;
        for cluster in clusters {
            let byte = cluster.byte_idx.value();
            let global = base + byte;
            let span = chunk.spans.partition_point(|span| span.end <= byte);
            let owner = text
                .length_ranges
                .iter()
                .rposition(|range| range.range().contains(&global));
            let spacing = owner
                .is_some_and(|id| text.length_ranges[id].adjustment() == LengthAdjust::Spacing);
            let boundary = text.length_breaks.binary_search(&global).is_ok();
            if spacing
                || boundary
                || previous.is_none_or(|(old_span, old_owner, old_byte)| {
                    span != old_span || owner != old_owner || byte <= old_byte
                })
            {
                group += 1;
            }
            result.push(group);
            previous = Some((span, owner, byte));
        }
        base += chunk.text.len();
    }
    result
}
