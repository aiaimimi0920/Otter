//! Preserve element-owned length constraints while serializing resolved styles.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn attributes(range: &TextLengthRange, xml: &mut XmlWriter) {
    xml.write_svg_attribute(AId::TextLength, &range.length());
    if range.adjustment() == LengthAdjust::SpacingAndGlyphs {
        xml.write_svg_attribute(AId::LengthAdjust, "spacingAndGlyphs");
    }
}

fn close_at(text: &Text, position: usize, stack: &mut Vec<usize>, xml: &mut XmlWriter) {
    while stack
        .last()
        .is_some_and(|&id| text.length_ranges[id].range().end <= position)
    {
        xml.end_element();
        stack.pop();
    }
}

pub(super) fn write(
    text: &Text,
    clip: bool,
    opt: &text_path_ids::Context<'_>,
    xml: &mut XmlWriter,
) {
    let total: usize = text.chunks.iter().map(|chunk| chunk.text.len()).sum();
    // A whole-text outer constraint belongs on text, so textPath stays a direct child.
    let root = text
        .length_ranges
        .first()
        .filter(|range| range.range() == (0..total));
    if let Some(root) = root {
        attributes(root, xml);
    }
    let mut starts: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut boundaries = BTreeSet::new();
    // Zero-valued positioning is observable through nested scale fragments.
    boundaries.extend(text.length_breaks.iter().copied());
    for (id, range) in text.length_ranges.iter().enumerate() {
        if id == 0 && root.is_some() {
            continue;
        }
        starts.entry(range.range().start).or_default().push(id);
        boundaries.insert(range.range().start);
        boundaries.insert(range.range().end);
    }
    let mut stack = Vec::new();
    let mut base = 0;
    let mut active_path = None;
    for chunk in &text.chunks {
        close_at(text, base, &mut stack, xml);
        let next_path = match &chunk.text_flow {
            TextFlow::Path(path) => Some(std::sync::Arc::as_ptr(path)),
            TextFlow::Linear => None,
        };
        if next_path != active_path {
            if active_path.is_some() {
                xml.end_element();
            }
            if let TextFlow::Path(path) = &chunk.text_flow {
                xml.start_svg_element(EId::TextPath);
                let prefix = opt.id_prefix.as_deref().unwrap_or_default();
                xml.write_attribute_fmt(
                    "xlink:href",
                    format_args!("#{}{}", prefix, opt.path_id(path)),
                );
                if path.start_offset != 0.0 {
                    xml.write_svg_attribute(AId::StartOffset, &path.start_offset);
                }
            }
            active_path = next_path;
        }
        for span in &chunk.spans {
            let start = base + span.start;
            let end = base + span.end;
            let mut cuts: Vec<_> = boundaries.range(start..end).copied().collect();
            if cuts.first() != Some(&start) {
                cuts.insert(0, start);
            }
            cuts.push(end);
            for piece in cuts.windows(2) {
                let (start, end) = (piece[0], piece[1]);
                close_at(text, start, &mut stack, xml);
                if let Some(ids) = starts.get(&start) {
                    for &id in ids {
                        xml.start_svg_element(EId::Tspan);
                        attributes(&text.length_ranges[id], xml);
                        if text.length_ranges[id].range().is_empty() {
                            xml.end_element();
                        } else {
                            stack.push(id);
                        }
                    }
                }
                let mut slice = span.clone();
                slice.start = start - base;
                slice.end = end - base;
                // The range wrapper owns the constraint; never duplicate it on a style leaf.
                slice.text_length = None;
                slice.length_adjust = LengthAdjust::Spacing;
                write_piece(chunk, &slice, text.right_to_left, clip, opt, xml);
            }
        }
        base += chunk.text.len();
    }
    close_at(text, total, &mut stack, xml);
    if active_path.is_some() {
        xml.end_element();
    }
}

fn write_piece(
    chunk: &TextChunk,
    span: &TextSpan,
    right_to_left: bool,
    clip: bool,
    opt: &WriteOptions,
    xml: &mut XmlWriter,
) {
    xml.start_svg_element(EId::Tspan);
    // Repeating x/y on every style leaf would create extra anchored chunks.
    if span.start == 0 {
        if let Some(x) = chunk.x {
            xml.write_svg_attribute(AId::X, &x);
        }
        if let Some(y) = chunk.y {
            xml.write_svg_attribute(AId::Y, &y);
        }
    }
    match chunk.anchor.for_direction(right_to_left) {
        TextAnchor::Start => xml.write_svg_attribute(AId::TextAnchor, "start"),
        TextAnchor::Middle => xml.write_svg_attribute(AId::TextAnchor, "middle"),
        TextAnchor::End => xml.write_svg_attribute(AId::TextAnchor, "end"),
    }
    let decorations: Vec<_> = [
        ("underline", &span.decoration.underline),
        ("line-through", &span.decoration.line_through),
        ("overline", &span.decoration.overline),
    ]
    .iter()
    .filter_map(|&(name, value)| value.as_ref().map(|value| (name, value)))
    .collect();
    for (name, deco) in &decorations {
        xml.start_svg_element(EId::Tspan);
        xml.write_svg_attribute(AId::TextDecoration, name);
        write_fill(&deco.fill, false, opt, xml);
        write_stroke(&deco.stroke, opt, xml);
    }
    write_span(clip, opt, xml, chunk, span);
    for _ in decorations {
        xml.end_element();
    }
    xml.end_element();
}
