//! Public boundary for Art parameter candidates: merge defaults and manual overrides, never admitting credentials or internal control fields.
use crate::graph_image::{Error, Output, MAX_PORTS};
use crate::graph_param_coercion::{coerce, Value, Widget};
pub const DISABLED: &str = "__DISABLED__";
pub struct Param<'a> {
    pub id: &'a str,
    pub default: Option<Output<'a>>,
    pub secret: bool,
    pub widget: Widget<'a>,
}
/// None represents undefined in the baseline; distinguish it from explicit null and do not automatically replace it with a default.
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
        // As in the baseline, manual entries ultimately override defaults, including explicit null and undeclared but noninternal entries.
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
    pub fn accepts_link(&self, name: &str) -> bool {
        self.linked_param(name).is_some()
    }
    fn linked_param(&self, name: &str) -> Option<&'a Param<'a>> {
        if self.excluded(name) {
            return None;
        }
        let param = self.declared.iter().rev().find(|param| param.id == name)?;
        (!self
            .manual
            .iter()
            .any(|(key, value)| *key == name && *value == Output::Image(DISABLED)))
        .then_some(param)
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
    /// Input must already have passed widget conversion; undeclared, secret, control, and manually disabled fields all reject connected overrides.
    /// Missing/empty formal outputs do not override the original value; subsequent field-conversion failures should likewise pass None.
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
    /// Swap only after candidate conversion completes; failure preserves the original value, and generated-text owners are freed with the parameter candidate.
    pub fn apply_widget_linked(
        &mut self,
        name: &str,
        widget: &Widget<'_>,
        value: Option<Output<'a>>,
    ) -> bool {
        let Some(param) = self.linked_param(name) else {
            return false;
        };
        self.store_linked(param.id, widget, value)
    }
    /// Resolve the upstream value only after admitting the target port.
    pub(super) fn apply_declared_linked(
        &mut self,
        name: &str,
        resolve: impl FnOnce() -> Option<Output<'a>>,
    ) -> bool {
        let Some(param) = self.linked_param(name) else {
            return false;
        };
        self.store_linked(param.id, &param.widget, resolve())
    }
    fn store_linked(
        &mut self,
        id: &'a str,
        widget: &Widget<'_>,
        value: Option<Output<'a>>,
    ) -> bool {
        let Some(value) = value.filter(|value| !matches!(value, Output::Null)) else {
            return false;
        };
        let Some(value) = coerce(widget, value) else {
            return false;
        };
        self.put_value(id, Some(value));
        true
    }
}
#[cfg(test)]
#[path = "graph_params_tests.rs"]
mod tests;
