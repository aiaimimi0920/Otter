//! Art 参数候选的公共边界：默认值与手动覆盖合并，凭据和内部控制字段永不进入候选。
use crate::graph_image::{Error, Output, MAX_PORTS};
use crate::graph_param_coercion::{coerce, Value, Widget};
pub const DISABLED: &str = "__DISABLED__";
pub struct Param<'a> {
    pub id: &'a str,
    pub default: Option<Output<'a>>,
    pub secret: bool,
    pub widget: Widget<'a>,
}
/// None 表示主干中的 undefined；与显式 null 区分，不能自动改成默认值。
pub struct Params<'a> {
    entries: Vec<(&'a str, Option<Value<'a>>)>,
    declared: &'a [Param<'a>],
    manual: &'a [(&'a str, Output<'a>)],
}
pub fn internal(name: &str) -> bool {
    name.starts_with("__exec_") || matches!(name, "__ui_resize" | "force_update")
}
impl<'a> Params<'a> {
    pub fn new(
        declared: &'a [Param<'a>],
        manual: &'a [(&'a str, Output<'a>)],
    ) -> Result<Self, Error> {
        if declared.len() > MAX_PORTS || manual.len() > MAX_PORTS {
            return Err(Error::Capacity);
        }
        if declared.iter().any(|param| param.id.len() > 256)
            || manual.iter().any(|(name, _)| name.len() > 256)
        {
            return Err(Error::Capacity);
        }
        if manual
            .iter()
            .enumerate()
            .any(|(index, (name, _))| manual[..index].iter().any(|(other, _)| other == name))
        {
            return Err(Error::InvalidOutput);
        }
        let mut result = Self {
            entries: Vec::new(),
            declared,
            manual,
        };
        result
            .entries
            .try_reserve(declared.len() + manual.len())
            .map_err(|_| Error::Allocation)?;
        for param in declared {
            if !result.excluded(param.id) {
                if let Some(value) = param.default {
                    crate::graph_image::validate_outputs(&[(param.id, value)])?;
                }
                let value = manual
                    .iter()
                    .find(|(name, _)| *name == param.id)
                    .map(|(_, value)| *value)
                    .filter(|value| !matches!(value, Output::Null))
                    .or(param.default);
                result.put(param.id, value);
            }
        }
        // 与主干相同：手动项最终覆盖默认值，包含显式 null 和未声明但非内部的项。
        for (name, value) in manual {
            if !result.excluded(name) {
                crate::graph_image::validate_outputs(&[(name, *value)])?;
                result.put(name, Some(*value));
            }
        }
        Ok(result)
    }
    fn excluded(&self, name: &str) -> bool {
        internal(name)
            || self
                .declared
                .iter()
                .any(|param| param.id == name && param.secret)
    }
    pub(crate) fn accepts_link(&self, name: &str) -> bool {
        !self.excluded(name)
            && self.declared.iter().any(|param| param.id == name)
            && !self
                .manual
                .iter()
                .any(|(key, value)| *key == name && *value == Output::Image(DISABLED))
    }
    fn put(&mut self, name: &'a str, value: Option<Output<'a>>) {
        self.put_value(name, value.map(Value::Borrowed));
    }
    fn put_value(&mut self, name: &'a str, value: Option<Value<'a>>) {
        if let Some(entry) = self.entries.iter_mut().find(|(key, _)| *key == name) {
            entry.1 = value;
        } else {
            self.entries.push((name, value));
        }
    }
    pub fn entries(&self) -> impl ExactSizeIterator<Item = (&str, Option<Output<'_>>)> {
        self.entries
            .iter()
            .map(|(name, value)| (*name, value.as_ref().map(Value::as_output)))
    }
    /// 输入须已经过 widget 转换；未声明、secret、控制字段和手动禁用项均拒绝连线覆盖。
    /// 缺失/空正式输出不覆盖原值，后续字段转换失败同样应传 None。
    pub fn apply_linked(&mut self, name: &str, value: Option<Output<'a>>) -> bool {
        self.apply_widget_linked(
            name,
            &Widget {
                name: "",
                min: None,
                max: None,
            },
            value,
        )
    }
    /// 转换候选完成后才交换；失败保留原值，生成文本的 owner 随参数候选释放。
    pub fn apply_widget_linked(
        &mut self,
        name: &str,
        widget: &Widget<'_>,
        value: Option<Output<'a>>,
    ) -> bool {
        if !self.accepts_link(name) {
            return false;
        }
        let Some(param) = self.declared.iter().rev().find(|param| param.id == name) else {
            return false;
        };
        let Some(value) = value.filter(|value| !matches!(value, Output::Null)) else {
            return false;
        };
        let Some(value) = coerce(widget, value) else {
            return false;
        };
        self.put_value(param.id, Some(value));
        true
    }
}
#[cfg(test)]
#[path = "graph_params_tests.rs"]
mod tests;
