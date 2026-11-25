//! Bind formal graph outputs to public-parameter candidates; each resolution is independent and does not alter sources, manual parameters, or the graph collection.
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
        // In the baseline, only Art capabilities provide parameter definitions; ordinary pins still retain noninternal manual entries.
        let declared = if self.nodes[index].kind == Kind::Art {
            declared
        } else {
            &[]
        };
        let mut resolved = Params::new(declared, manual)?;
        for link in self.links.iter().filter(|link| &link.to_unit == id) {
            // Resolve each edge independently. Missing values/conversion failures preserve the candidate; later valid edges override earlier values.
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
