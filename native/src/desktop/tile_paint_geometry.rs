//! Project wall geometry once per layout/output change, without retaining frames or pixels.
use crate::{tile_geometry, tile_shared::Frame, tile_wire::Rect};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};

pub(super) enum Geometry {
    Grid(usize),
    Placed(Vec<Placed>),
}

pub(super) struct Placed {
    pub(super) index: usize,
    pub(super) transform: [f32; 6],
    pub(super) crop: Rect,
}

impl Geometry {
    pub(super) fn same_inputs(previous: &Frame, next: &Frame) -> bool {
        previous.size == next.size
            && previous.key.endpoint == next.key.endpoint
            && previous.layout == next.layout
    }

    pub(super) fn project(frame: &Frame) -> Result<Self> {
        let index = frame
            .layout
            .tiles
            .iter()
            .position(|t| t.endpoint_id == frame.key.endpoint)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Tile missing"))?;
        let tile = &frame.layout.tiles[index];
        let placements = &frame.layout.placements;
        if placements.is_empty() {
            return Ok(Self::Grid(index));
        }
        let (w, h) = (frame.size.width as f32, frame.size.height as f32);
        let mut projected: Vec<_> = placements
            .iter()
            .enumerate()
            .filter_map(|(index, placement)| {
                let projection = tile_geometry::project(tile, placement)?;
                let [q0, q1, q3] = projection.quad;
                Some(Placed {
                    index,
                    transform: [
                        (q1[0] - q0[0]) as f32 * w,
                        (q1[1] - q0[1]) as f32 * h,
                        (q3[0] - q0[0]) as f32 * w,
                        (q3[1] - q0[1]) as f32 * h,
                        q0[0] as f32 * w,
                        q0[1] as f32 * h,
                    ],
                    crop: projection.crop,
                })
            })
            .collect();
        projected.sort_by(|a, b| {
            let (a, b) = (&placements[a.index], &placements[b.index]);
            a.z_index
                .cmp(&b.z_index)
                .then(a.placement_id.cmp(&b.placement_id))
        });
        Ok(Self::Placed(projected))
    }
}
