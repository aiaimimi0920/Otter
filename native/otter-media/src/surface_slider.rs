//! Slider 的有限数值/步长合同；复用浏览器 range 归一规则，不持有 draft 或动作权限。
use crate::surface_scene::{Kind, Node};
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Kind,
    Precision,
    Value,
}
#[derive(Debug, Clone, Copy)]
pub struct Range {
    min: f64,
    max: f64,
    step: f64,
    last: f64,
    source: f64,
}
fn tolerance(value: f64) -> f64 {
    (8. * f64::EPSILON * value.abs().max(1.)).min(1e-7)
}
fn nearest(value: f64) -> f64 {
    let lower = value.floor();
    // 十进制浮点误差不能把精确中点改成向下舍入；range 中点必须取较大的值。
    if value - lower >= 0.5 - tolerance(value) {
        lower + 1.
    } else {
        lower
    }
}
impl Range {
    pub fn parse(node: &Node) -> Result<Self, Error> {
        if node.kind() != Kind::Slider {
            return Err(Error::Kind);
        }
        let number = |key, fallback| {
            node.props()
                .get(key)
                .and_then(serde_json::Value::as_f64)
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        };
        let min = number("min", 0.);
        let max = number("max", 100.).max(min);
        let declared_step = number("step", 1.);
        let step = if declared_step > 0. {
            declared_step
        } else {
            1.
        };
        let span = max - min;
        let quotient = span / step;
        // 不截断为 i32 网格或静默改步长；无法可靠区分的极端步距显式拒绝，留给后续扩展。
        if !quotient.is_finite()
            || quotient >= (1_u64 << 52) as f64
            || (span > 0. && min + step == min)
        {
            return Err(Error::Precision);
        }
        let rounded = quotient.round();
        let last = if (quotient - rounded).abs() <= tolerance(quotient) {
            rounded
        } else {
            quotient.floor()
        };
        let mut range = Self {
            min,
            max,
            step,
            last,
            source: min,
        };
        // JSX 显式 value 缺省为 0，不采用裸 HTML range 缺省中点 50。
        range.source = range.normalize(number("value", 0.))?;
        Ok(range)
    }
    pub fn min(&self) -> f64 {
        self.min
    }
    pub fn max(&self) -> f64 {
        self.max
    }
    pub fn step(&self) -> f64 {
        self.step
    }
    pub fn source(&self) -> f64 {
        self.source
    }
    fn value_at(&self, index: f64) -> f64 {
        (self.min + index.clamp(0., self.last) * self.step).clamp(self.min, self.max)
    }
    pub fn normalize(&self, value: f64) -> Result<f64, Error> {
        if !value.is_finite() {
            return Err(Error::Value);
        }
        let index = (value.clamp(self.min, self.max) - self.min) / self.step;
        Ok(self.value_at(nearest(index)))
    }
    /// 输入位置必须来自当前控件几何；不接受 NaN 或无限值，不产生无界迭代。
    pub fn at_fraction(&self, fraction: f64) -> Result<f64, Error> {
        if !fraction.is_finite() {
            return Err(Error::Value);
        }
        self.normalize(self.min + fraction.clamp(0., 1.) * (self.max - self.min))
    }
    pub fn fraction(&self, value: f64) -> Result<f64, Error> {
        let value = self.normalize(value)?;
        Ok(if self.max == self.min {
            0.
        } else {
            (value - self.min) / (self.max - self.min)
        })
    }
    pub fn advance(&self, value: f64, up: bool) -> Result<f64, Error> {
        let value = self.normalize(value)?;
        let index = nearest((value - self.min) / self.step);
        Ok(self.value_at(index + if up { 1. } else { -1. }))
    }
}
#[cfg(test)]
#[path = "surface_slider_tests.rs"]
mod tests;
