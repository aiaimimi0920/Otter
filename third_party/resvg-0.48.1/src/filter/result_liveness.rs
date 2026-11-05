// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Last reference positions bound retention without changing result resolution.
use std::collections::HashMap;
use usvg::filter::{Input, Kind, Primitive};

pub(super) struct LastUses<'a>(HashMap<&'a str, usize>);

impl<'a> LastUses<'a> {
    pub fn new(primitives: &'a [Primitive]) -> Self {
        let mut uses = HashMap::new();
        for (index, primitive) in primitives.iter().enumerate() {
            visit(primitive.kind(), |input| {
                if let Input::Reference(name) = input {
                    uses.insert(name.as_str(), index);
                }
            });
        }
        Self(uses)
    }

    pub fn needed_after(&self, name: &str, index: usize) -> bool {
        self.0.get(name).is_some_and(|last| *last > index)
    }
}

fn visit<'a>(kind: &'a Kind, mut input: impl FnMut(&'a Input)) {
    match kind {
        Kind::Blend(fe) => {
            input(fe.input1());
            input(fe.input2());
        }
        Kind::Composite(fe) => {
            input(fe.input1());
            input(fe.input2());
        }
        Kind::DisplacementMap(fe) => {
            input(fe.input1());
            input(fe.input2());
        }
        Kind::Merge(fe) => {
            for item in fe.inputs() {
                input(item);
            }
        }
        Kind::ColorMatrix(fe) => input(fe.input()),
        Kind::ComponentTransfer(fe) => input(fe.input()),
        Kind::ConvolveMatrix(fe) => input(fe.input()),
        Kind::DiffuseLighting(fe) => input(fe.input()),
        Kind::DropShadow(fe) => input(fe.input()),
        Kind::GaussianBlur(fe) => input(fe.input()),
        Kind::Morphology(fe) => input(fe.input()),
        Kind::Offset(fe) => input(fe.input()),
        Kind::SpecularLighting(fe) => input(fe.input()),
        Kind::Tile(fe) => input(fe.input()),
        Kind::Flood(_) | Kind::Image(_) | Kind::Turbulence(_) => {}
    }
}
