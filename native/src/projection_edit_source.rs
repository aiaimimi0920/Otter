//! 源端不可变basis；PNG沿sender codec/预算，journal持有大资源，canonical只持轻量View。
use super::{
    view::{Size, View},
    Basis, Error, Objects,
};
use crate::{
    projection_delivery::sender::{self, Frame},
    session_vector_write::Mapping,
    viewport::Point,
};
use serde::{Deserialize, Serialize, Serializer};
use std::sync::{atomic::AtomicBool, Arc};
#[path = "projection_edit_source_wire.rs"]
mod wire;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Layout {
    width: f64,
    height: f64,
    offset_x: f64,
    offset_y: f64,
}
impl Layout {
    pub(crate) fn new(
        width: f64,
        height: f64,
        offset_x: f64,
        offset_y: f64,
    ) -> Result<Self, Error> {
        let value = Self {
            width,
            height,
            offset_x,
            offset_y,
        };
        value.size()?;
        value.mapping()?;
        Ok(value)
    }
    pub(crate) fn size(self) -> Result<Size, Error> {
        Size::new(self.width, self.height).map_err(|_| Error::Basis)
    }
    pub(crate) fn mapping(self) -> Result<Mapping, Error> {
        if !self.offset_x.is_finite()
            || !self.offset_y.is_finite()
            || self.offset_x.abs() > 1_000_000.0
            || self.offset_y.abs() > 1_000_000.0
        {
            return Err(Error::Basis);
        }
        Mapping::from_components(
            1.0,
            Point {
                x: self.offset_x as f32,
                y: self.offset_y as f32,
            },
        )
        .map_err(|_| Error::Basis)
    }
}
struct Data {
    anchor: String,
    session: String,
    layout: Layout,
    initial: Objects,
    snapshot: sender::snapshot::Snapshot,
}
#[derive(Clone)]
pub(crate) struct Frozen(Arc<Data>);
impl Frozen {
    pub(crate) fn new(
        anchor: &str,
        session: &str,
        frame: Frame,
        layout: Layout,
        initial: impl FnOnce(&Basis, Layout) -> Result<Objects, Error>,
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        let snapshot =
            sender::snapshot::Snapshot::from_png(&frame.png, frame.width, frame.height, cancel)
                .map_err(|_| Error::Basis)?;
        let basis = Basis::new(&snapshot.digest, snapshot.width, snapshot.height)?;
        let initial = initial(&basis, layout)?;
        let value = Self(Arc::new(Data {
            anchor: anchor.into(),
            session: session.into(),
            layout,
            initial,
            snapshot,
        }));
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if !super::projection(self.anchor()) || !super::session(self.session()) {
            return Err(Error::Identity);
        }
        self.layout().mapping()?;
        self.view()?;
        Ok(())
    }
    pub(crate) fn basis(&self) -> Basis {
        Basis::new(
            &self.0.snapshot.digest,
            self.0.snapshot.width,
            self.0.snapshot.height,
        )
        .unwrap()
    }
    pub(crate) fn view(&self) -> Result<View, Error> {
        View::from_basis(&self.basis(), self.layout().size()?, self.0.initial.clone())
            .map_err(|_| Error::Annotation)
    }
    pub(crate) fn anchor(&self) -> &str {
        &self.0.anchor
    }
    pub(crate) fn session(&self) -> &str {
        &self.0.session
    }
    pub(crate) fn layout(&self) -> Layout {
        self.0.layout
    }
    pub(crate) fn initial(&self) -> &Objects {
        &self.0.initial
    }
    pub(crate) fn same_basis(&self, other: &Self) -> bool {
        self.basis() == other.basis()
    }
    pub(crate) fn frame(
        &self,
        kind: sender::Kind,
        cancel: &AtomicBool,
    ) -> Result<Frame, sender::Error> {
        Ok(Frame {
            png: self.0.snapshot.png(cancel)?,
            width: self.0.snapshot.width,
            height: self.0.snapshot.height,
            kind,
        })
    }
}
impl PartialEq for Frozen {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.same_basis(other)
                && self.layout() == other.layout()
                && self.anchor() == other.anchor()
                && self.session() == other.session()
                && self.initial() == other.initial())
    }
}
impl std::fmt::Debug for Frozen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrozenProjectionBasis")
            .field("anchor", &self.anchor())
            .finish_non_exhaustive()
    }
}
impl Serialize for Frozen {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Fields<'a> {
            anchor: &'a str,
            session: &'a str,
            layout: Layout,
            initial: &'a Objects,
            snapshot: &'a sender::snapshot::Snapshot,
        }
        Fields {
            anchor: self.anchor(),
            session: self.session(),
            layout: self.layout(),
            initial: self.initial(),
            snapshot: &self.0.snapshot,
        }
        .serialize(serializer)
    }
}
#[cfg(test)]
#[path = "projection_edit_source_tests.rs"]
mod tests;
