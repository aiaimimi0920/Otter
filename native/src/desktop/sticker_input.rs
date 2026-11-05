//! 文档独立图片输入owner；原画布、Art参数与手动输入不共享可变状态或授权身份。
use serde_json::{Map, Value};
use std::{
    cell::{Cell, RefCell},
    sync::{atomic::AtomicBool, Arc},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
#[path = "sticker_input_asset.rs"]
mod asset;
pub(super) use asset::Asset;
#[path = "sticker_input_actions.rs"]
mod actions;
pub(super) use actions::{apply, Action};

#[derive(Clone)]
pub(super) struct Stamp(Arc<()>);
impl PartialEq for Stamp {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Clone, Default)]
pub(super) struct Config {
    pub asset: Option<Arc<Asset>>,
    pub disabled: bool,
    parameters: Arc<Map<String, Value>>,
    identity: Arc<()>,
}
impl Config {
    pub fn stamp(&self) -> Stamp {
        Stamp(self.identity.clone())
    }
    pub fn configured(&self) -> bool {
        self.asset.is_some() || self.disabled || !self.parameters.is_empty()
    }
    pub fn restore(raw: Option<&serde_json::value::RawValue>, cancel: &AtomicBool) -> Result<Self> {
        let mut parameters =
            crate::sticker_image_params::read(raw).map_err(|e| Error::new(E_INVALIDARG, e))?;
        let disabled =
            parameters.get("image").and_then(Value::as_str) == Some(crate::graph_params::DISABLED);
        let asset = parameters
            .get("image_path")
            .and_then(Value::as_str)
            .filter(|uri| !uri.is_empty())
            .map(|uri| {
                Asset::restore(
                    uri,
                    parameters
                        .get("image_filename")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    cancel,
                )
            })
            .transpose()?;
        parameters.remove("image_path");
        parameters.remove("image_filename");
        Ok(Self {
            asset,
            disabled,
            parameters: Arc::new(parameters),
            identity: Arc::new(()),
        })
    }
    /// 在保存worker中组装；只有拥有的键被替换，clear不能从retained original复活旧图。
    pub fn parameters(&self) -> Map<String, Value> {
        let mut values = self.parameters.as_ref().clone();
        if let Some(asset) = &self.asset {
            values.insert("image_path".into(), asset.uri.as_str().into());
            values.insert("image_filename".into(), asset.filename.as_str().into());
        }
        if self.disabled {
            values.insert("image".into(), crate::graph_params::DISABLED.into());
        } else if values.get("image").and_then(Value::as_str) == Some(crate::graph_params::DISABLED)
        {
            values.remove("image");
        }
        values
    }
    /// 发布前按保存形状计量；借用未知字段，绝不在UI线程复制大图URI。
    fn validate(&self) -> Result<()> {
        let empty = Value::Null;
        let filename = self
            .asset
            .as_ref()
            .map(|a| Value::String(a.filename.clone()));
        let disabled = Value::String(crate::graph_params::DISABLED.into());
        let image = if self.disabled {
            Some(&disabled)
        } else {
            self.parameters
                .get("image")
                .filter(|v| v.as_str() != Some(crate::graph_params::DISABLED))
        };
        let entries = self
            .parameters
            .iter()
            .filter(|(key, _)| key.as_str() != "image")
            .map(|(key, value)| (key.as_str(), Some(value)))
            .chain(self.asset.as_ref().map(|_| ("image_path", Some(&empty))))
            .chain(
                filename
                    .as_ref()
                    .map(|value| ("image_filename", Some(value))),
            )
            .chain(image.map(|value| ("image", Some(value))));
        crate::loom_parameter_state::validate(entries)
            .map_err(|_| Error::new(E_INVALIDARG, "图片输入修改超过参数结构或容量限制"))
    }
}
#[derive(Default)]
pub(super) struct Control {
    value: RefCell<Config>,
    changed: Cell<bool>,
}
impl Control {
    pub fn snapshot(&self) -> Config {
        self.value.borrow().clone()
    }
    pub fn restore(&self, config: Config) {
        *self.value.borrow_mut() = config;
    }
    pub fn take_changed(&self) -> bool {
        self.changed.replace(false)
    }
    pub fn set_asset(&self, asset: Option<Arc<Asset>>) -> Result<()> {
        let mut next = self.snapshot();
        next.asset = asset;
        self.commit(next)
    }
    pub fn toggle(&self) -> Result<()> {
        let mut next = self.snapshot();
        next.disabled = !next.disabled;
        self.commit(next)
    }
    fn commit(&self, mut next: Config) -> Result<()> {
        next.validate()?;
        next.identity = Arc::new(());
        *self.value.borrow_mut() = next;
        self.changed.set(true);
        Ok(())
    }
}

#[cfg(test)]
#[path = "sticker_input_tests.rs"]
pub(super) mod tests;
