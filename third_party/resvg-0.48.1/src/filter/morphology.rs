// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::ImageRefMut;
use rgb::RGBA8;
use std::collections::VecDeque;
use usvg::filter::MorphologyOperator;

/// Separable rectangular min/max over premultiplied channels, with transparent
/// pixels outside the image. One image buffer and one bounded line queue; time
/// is linear in pixel count rather than proportional to the radius area.
pub fn apply(operator: MorphologyOperator, rx: f32, ry: f32, src: ImageRefMut) {
    let width = src.width as usize;
    let height = src.height as usize;
    // Beyond a full axis, dilation sees every pixel and erosion sees transparent
    // padding. Clamp the radius, not the window width or its center.
    let rx = (rx.round() as usize).min(width);
    let ry = (ry.round() as usize).min(height);
    let mut buffer = vec![RGBA8::default(); src.data.len()];
    let mut queue = VecDeque::with_capacity(width.max(height));
    axis(
        src.data,
        &mut buffer,
        width,
        height,
        rx,
        true,
        operator,
        &mut queue,
    );
    axis(
        &buffer, src.data, width, height, ry, false, operator, &mut queue,
    );
}

#[allow(clippy::too_many_arguments)]
fn axis(
    source: &[RGBA8],
    target: &mut [RGBA8],
    width: usize,
    height: usize,
    radius: usize,
    horizontal: bool,
    operator: MorphologyOperator,
    queue: &mut VecDeque<(usize, u8)>,
) {
    let erode = operator == MorphologyOperator::Erode;
    let (length, lines, stride) = if horizontal {
        (width, height, 1)
    } else {
        (height, width, width)
    };
    for line in 0..lines {
        let base = if horizontal { line * width } else { line };
        for channel in 0..4 {
            queue.clear();
            let mut next = 0;
            for center in 0..length {
                let left = center.saturating_sub(radius);
                let right = center.saturating_add(radius).min(length - 1);
                while next <= right {
                    let value = get(source[base + next * stride], channel);
                    while queue
                        .back()
                        .is_some_and(|&(_, old)| if erode { old >= value } else { old <= value })
                    {
                        queue.pop_back();
                    }
                    queue.push_back((next, value));
                    next += 1;
                }
                while queue.front().is_some_and(|&(index, _)| index < left) {
                    queue.pop_front();
                }
                let outside = center < radius || radius > length - 1 - center;
                let value = if erode && outside {
                    0
                } else {
                    queue.front().unwrap().1
                };
                set(&mut target[base + center * stride], channel, value);
            }
        }
    }
}

fn get(pixel: RGBA8, channel: usize) -> u8 {
    match channel {
        0 => pixel.r,
        1 => pixel.g,
        2 => pixel.b,
        _ => pixel.a,
    }
}
fn set(pixel: &mut RGBA8, channel: usize, value: u8) {
    match channel {
        0 => pixel.r = value,
        1 => pixel.g = value,
        2 => pixel.b = value,
        _ => pixel.a = value,
    }
}

#[cfg(test)]
#[path = "morphology_tests.rs"]
mod tests;
