//! 私有 Canvas 绘制提交的数据合同；每次提交都在调用 D2D 前完成类型、有限数和容量验证。
use crate::surface_javascript_wire::{canvas, invalid};
use serde::Deserialize;
use std::io;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Batch {
    pub id: String,
    pub size: [u32; 2],
    pub reset: bool,
    pub commands: Vec<Draw>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct State {
    pub transform: [f32; 6],
    pub color: String,
    pub alpha: f32,
    pub width: f32,
    pub cap: String,
    pub join: String,
    pub miter: f32,
    pub dash: Vec<f32>,
    pub dash_offset: f32,
    pub font: String,
    pub align: String,
    pub baseline: String,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum Segment {
    Move {
        point: [f32; 2],
    },
    Line {
        point: [f32; 2],
    },
    Curve {
        points: [[f32; 2]; 3],
    },
    Arc {
        center: [f32; 2],
        radius: f32,
        start: f32,
        end: f32,
        ccw: bool,
    },
    Close,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(super) enum Draw {
    Image {
        id: String,
        source: [f32; 4],
        destination: [f32; 4],
        transform: [f32; 6],
        alpha: f32,
        smooth: bool,
    },
    Clear {
        rect: [f32; 4],
        transform: [f32; 6],
    },
    Rect {
        rect: [f32; 4],
        state: State,
        stroke: bool,
    },
    Path {
        path: Vec<Segment>,
        state: State,
        stroke: bool,
        evenodd: bool,
    },
    Text {
        text: String,
        point: [f32; 2],
        state: State,
    },
}
pub(super) fn finite(values: &[f32]) -> bool {
    values
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_048_576.)
}
pub(super) fn font(value: &str) -> io::Result<(f32, &str)> {
    if value.len() > 256 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    let (size, families) = value.split_once("px ").ok_or_else(invalid)?;
    let size: f32 = size.parse().map_err(|_| invalid())?;
    let family = families
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(['\'', '"']);
    if !size.is_finite() || !(1. ..=256.).contains(&size) || family.is_empty() {
        return Err(invalid());
    }
    Ok((
        size,
        match family {
            "sans-serif" => "Segoe UI",
            "monospace" => "Consolas",
            "serif" => "Times New Roman",
            _ => family,
        },
    ))
}
impl State {
    pub(super) fn validate(&self) -> io::Result<()> {
        if !finite(&self.transform)
            || !self.alpha.is_finite()
            || !(0. ..=1.).contains(&self.alpha)
            || !self.width.is_finite()
            || !(0.001..=8192.).contains(&self.width)
            || !self.miter.is_finite()
            || !(0.001..=8192.).contains(&self.miter)
            || !finite(&[self.dash_offset])
            || self.dash.len() > 64
            || self
                .dash
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=8192.).contains(v))
            || !matches!(self.cap.as_str(), "butt" | "round" | "square")
            || !matches!(self.join.as_str(), "miter" | "round" | "bevel")
            || !matches!(
                self.align.as_str(),
                "left" | "right" | "center" | "start" | "end"
            )
            || !matches!(
                self.baseline.as_str(),
                "alphabetic" | "top" | "middle" | "bottom"
            )
            || crate::surface_color::Color::parse(&self.color, crate::surface_color::Color::BLACK)
                .is_none()
        {
            return Err(invalid());
        }
        font(&self.font)?;
        Ok(())
    }
}
impl Draw {
    pub(super) fn validate(&self) -> io::Result<()> {
        match self {
            Self::Image {
                id,
                source,
                destination,
                transform,
                alpha,
                ..
            } => {
                if !crate::surface_javascript_event::safe(id)
                    || !finite(source)
                    || !finite(destination)
                    || !finite(transform)
                    || !alpha.is_finite()
                    || !(0. ..=1.).contains(alpha)
                {
                    return Err(invalid());
                }
            }
            Self::Clear { rect, transform } => {
                // D2D 的 axis clip 不能表达旋转 clearRect；未接几何擦除前明确拒绝。
                if !finite(rect) || !finite(transform) || transform[1] != 0. || transform[2] != 0. {
                    return Err(invalid());
                }
            }
            Self::Rect { rect, state, .. } => {
                if !finite(rect) {
                    return Err(invalid());
                }
                state.validate()?;
            }
            Self::Path { path, state, .. } => {
                state.validate()?;
                if path.len() > 4096 {
                    return Err(invalid());
                }
                for segment in path {
                    let valid = match segment {
                        Segment::Move { point } | Segment::Line { point } => finite(point),
                        Segment::Curve { points } => points.iter().all(|p| finite(p)),
                        Segment::Arc {
                            center,
                            radius,
                            start,
                            end,
                            ..
                        } => finite(center) && finite(&[*radius, *start, *end]) && *radius >= 0.,
                        Segment::Close => true,
                    };
                    if !valid {
                        return Err(invalid());
                    }
                }
            }
            Self::Text { text, point, state } => {
                state.validate()?;
                if text.len() > 8192 || text.contains('\0') || !finite(point) {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
}
impl Batch {
    pub(super) fn validate(&self) -> io::Result<()> {
        canvas::bytes(self.size)?;
        if !crate::surface_javascript_event::safe(&self.id) || self.commands.len() > 8192 {
            return Err(invalid());
        }
        for command in &self.commands {
            command.validate()?;
        }
        Ok(())
    }
}
