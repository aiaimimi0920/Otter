//! 正式执行准备事务：准入、图片和公共参数全部成功才返回候选，不提前创建请求或修改 UI。
use super::*;
use crate::{
    graph_execution_config::{Config, Runtime, Skip, Trigger},
    graph_params::{internal, Param, Params, DISABLED},
};
pub enum SkipReason<'a> {
    MissingTarget,
    NotArt,
    Gate(Skip),
    MissingImages(Vec<&'a str>),
}
pub enum Preparation<'a> {
    Skipped(SkipReason<'a>),
    Ready(Prepared<'a>),
}
pub struct DispatchOptions {
    pub config: Config,
    pub runtime: Runtime,
    pub trigger: Trigger,
}
pub struct Prepared<'a> {
    params: Params<'a>,
    pub images: Vec<(&'a str, &'a str)>,
    pub disabled: Vec<&'a str>,
}
impl Prepared<'_> {
    /// 输出仅包含活动公共参数，不复制文本；禁止直接序列化 Params 的内部声明或手动缓存。
    pub fn active_params(&self) -> impl Iterator<Item = (&str, Option<Output<'_>>)> {
        self.params
            .entries()
            .filter(|(_, value)| *value != Some(Output::Image(DISABLED)))
    }
}
impl<'a> Graph<'a> {
    pub fn prepare_dispatch(
        &self,
        id: &UnitId,
        declared: &'a [Param<'a>],
        manual: &'a [(&'a str, Output<'a>)],
        options: DispatchOptions,
    ) -> Result<Preparation<'a>, Error> {
        let DispatchOptions {
            config,
            runtime,
            trigger,
        } = options;
        let Some(index) = self.index(id) else {
            return Ok(Preparation::Skipped(SkipReason::MissingTarget));
        };
        if self.nodes[index].kind != Kind::Art {
            return Ok(Preparation::Skipped(SkipReason::NotArt));
        }
        if let Err(reason) = config.gate(trigger, runtime) {
            return Ok(Preparation::Skipped(SkipReason::Gate(reason)));
        }
        let images = self.execution_images(id, manual)?;
        let missing = self.missing_execution_images(id, manual)?;
        if trigger != Trigger::Manual && !missing.is_empty() {
            return Ok(Preparation::Skipped(SkipReason::MissingImages(missing)));
        }
        let params = self.effective_params(id, declared, manual)?;
        let mut disabled = Vec::new();
        for (name, value) in manual {
            // 比主干更严格：secret 参数名也不加入公开 disabled 列表。
            if *value == Output::Image(DISABLED)
                && !internal(name)
                && !declared
                    .iter()
                    .any(|param| param.id == *name && param.secret)
            {
                disabled.try_reserve(1).map_err(|_| Error::Allocation)?;
                disabled.push(*name);
            }
        }
        Ok(Preparation::Ready(Prepared {
            params,
            images,
            disabled,
        }))
    }
}
#[cfg(test)]
#[path = "graph_dispatch_tests.rs"]
mod tests;
