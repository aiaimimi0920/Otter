//! 将正式图输出绑定到公共参数候选；每次解析独立，不修改来源、手动参数或图集合。
use super::*;
use crate::graph_params::{Param, Params};
impl<'a> Graph<'a> {
    pub fn effective_params(
        &self,
        id: &UnitId,
        declared: &'a [Param<'a>],
        manual: &'a [(&'a str, Output<'a>)],
    ) -> Result<Params<'a>, Error> {
        let Some(index) = self.index(id) else {
            return Params::new(&[], &[]);
        };
        // 主干只有 Art 能力提供参数定义；普通钉图仍保留非内部手动项。
        let declared = if self.nodes[index].kind == Kind::Art {
            declared
        } else {
            &[]
        };
        let mut resolved = Params::new(declared, manual)?;
        for link in self.links.iter().filter(|link| &link.to_unit == id) {
            // 每条边独立解析。无值/转换失败保留候选；后来的有效边覆盖较早的值。
            resolved.apply_declared_linked(&link.to_port, || {
                let port = if link.from_port.is_empty() {
                    "output"
                } else {
                    &link.from_port
                };
                self.output(&link.from_unit, port)
            });
        }
        Ok(resolved)
    }
}
#[cfg(test)]
#[path = "graph_effective_params_tests.rs"]
mod tests;
