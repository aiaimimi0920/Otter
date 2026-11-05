//! Leaf span construction stays outside recursive traversal stack frames.
use super::*;

#[inline(never)]
pub(super) fn collect(
    parent: SvgNode,
    child: SvgNode,
    pos_list: &[CharacterPosition],
    state: &converter::State,
    cache: &mut converter::Cache,
    iter_state: &mut IterState,
) {
    if !parent.is_visible_element(state.opt) {
        iter_state.chars_count += child.text().chars().count();
        return;
    }

    let mut anchor: TextAnchor = parent.find_attribute(AId::TextAnchor).unwrap_or_default();
    // Layout consumes a physical anchor; writer restores a logical anchor.
    // This handles anchor alignment, not unicode-bidi embedding/override rules.
    if parent.find_attribute::<&str>(AId::Direction) == Some("rtl") {
        anchor = anchor.for_direction(true);
    }

    // TODO: what to do when <= 0? UB?
    let font_size = crate::parser::units::resolve_font_size(parent, state);
    let font_size = match NonZeroPositiveF32::new(font_size) {
        Some(n) => n,
        None => {
            // Skip this span.
            iter_state.chars_count += child.text().chars().count();
            return;
        }
    };

    let font = convert_font(parent, state);

    let raw_paint_order: svgtypes::PaintOrder =
        parent.find_attribute(AId::PaintOrder).unwrap_or_default();
    let paint_order = super::converter::svg_paint_order_to_usvg(raw_paint_order);

    let mut dominant_baseline = parent
        .find_attribute(AId::DominantBaseline)
        .unwrap_or_default();

    // `no-change` means "use parent".
    if dominant_baseline == DominantBaseline::NoChange {
        dominant_baseline = parent
            .parent_element()
            .unwrap()
            .find_attribute(AId::DominantBaseline)
            .unwrap_or_default();
    }

    let mut apply_kerning = true;
    #[allow(clippy::if_same_then_else)]
    if parent.resolve_length(AId::Kerning, state, -1.0) == 0.0 {
        apply_kerning = false;
    } else if parent.find_attribute::<&str>(AId::FontKerning) == Some("none") {
        apply_kerning = false;
    }

    let font_optical_sizing = match parent.find_attribute::<&str>(AId::FontOpticalSizing) {
        Some("none") => crate::FontOpticalSizing::None,
        _ => crate::FontOpticalSizing::Auto, // "auto" or missing (browser default)
    };

    let mut text_length = parent.try_convert_length(AId::TextLength, Units::UserSpaceOnUse, state);
    // Negative values should be ignored.
    if let Some(n) = text_length {
        if n < 0.0 {
            text_length = None;
        }
    }

    let visibility: Visibility = parent.find_attribute(AId::Visibility).unwrap_or_default();

    let span = TextSpan {
        start: 0,
        end: 0,
        fill: style::resolve_fill(parent, true, state, cache),
        stroke: style::resolve_stroke(parent, true, state, cache),
        paint_order,
        font,
        font_size,
        small_caps: parent.find_attribute::<&str>(AId::FontVariant) == Some("small-caps"),
        apply_kerning,
        font_optical_sizing,
        decoration: resolve_decoration(parent, state, cache),
        visible: visibility == Visibility::Visible,
        dominant_baseline,
        alignment_baseline: parent
            .find_attribute(AId::AlignmentBaseline)
            .unwrap_or_default(),
        baseline_shift: convert_baseline_shift(parent, state),
        letter_spacing: parent.resolve_length(AId::LetterSpacing, state, 0.0),
        word_spacing: parent.resolve_length(AId::WordSpacing, state, 0.0),
        text_length,
        length_adjust: parent.find_attribute(AId::LengthAdjust).unwrap_or_default(),
    };

    let mut is_new_span = true;
    let has_rotation = parent
        .find_attribute::<Vec<f32>>(AId::Rotate)
        .is_some_and(|v| !v.is_empty());
    for c in child.text().chars() {
        let char_len = c.len_utf8();
        let position = pos_list[iter_state.chars_count];
        // Explicit zero offsets/rotations still split positioning fragments.
        if position.dx.is_some() || position.dy.is_some() || has_rotation {
            iter_state.length_breaks.push(iter_state.retained_bytes);
        }

        // Create a new chunk if:
        // - this is the first span (yes, position can be None)
        // - text character has an absolute coordinate assigned to it (via x/y attribute)
        // - `c` is the first char of the `textPath`
        // - `c` is the first char after `textPath`
        let is_new_chunk = pos_list[iter_state.chars_count].x.is_some()
            || pos_list[iter_state.chars_count].y.is_some()
            || iter_state.split_chunk
            || iter_state.chunks.is_empty();

        iter_state.split_chunk = false;

        if is_new_chunk {
            iter_state.chunk_bytes_count = 0;

            let mut span2 = span.clone();
            span2.start = 0;
            span2.end = char_len;

            iter_state.chunks.push(TextChunk {
                x: pos_list[iter_state.chars_count].x,
                y: pos_list[iter_state.chars_count].y,
                anchor,
                spans: vec![span2],
                text_flow: iter_state.text_flow.clone(),
                text: c.to_string(),
            });
        } else if is_new_span {
            // Add this span to the last text chunk.
            let mut span2 = span.clone();
            span2.start = iter_state.chunk_bytes_count;
            span2.end = iter_state.chunk_bytes_count + char_len;

            if let Some(chunk) = iter_state.chunks.last_mut() {
                chunk.text.push(c);
                chunk.spans.push(span2);
            }
        } else {
            // Extend the last span.
            if let Some(chunk) = iter_state.chunks.last_mut() {
                chunk.text.push(c);
                if let Some(span) = chunk.spans.last_mut() {
                    debug_assert_ne!(span.end, 0);
                    span.end += char_len;
                }
            }
        }

        is_new_span = false;
        iter_state.chars_count += 1;
        iter_state.chunk_bytes_count += char_len;
        iter_state.retained_bytes += char_len;
    }
}
