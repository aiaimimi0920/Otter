//! Dense coverage for small erasures; lazy 64x64 tiles and dirty row spans for large regions.

use crate::{byte_budget::Lease, erase::EraseError, pixels};

const TILE: usize = 64;
const TILE_CELLS: usize = TILE * TILE;
const TILE_BYTES: usize = TILE_CELLS * std::mem::size_of::<u16>();

pub(super) enum Mask {
    Dense {
        region: [u32; 4],
        cells: Vec<u16>,
        _lease: Lease,
        touched: bool,
    },
    Sparse(SparseMask),
}
impl Mask {
    pub(super) fn new(region: [u32; 4], output_bytes: usize) -> Result<Self, EraseError> {
        let width = (region[2] - region[0]) as usize;
        let height = (region[3] - region[1]) as usize;
        let cells = width * height;
        // The image was validated before clipping. Include edge padding and row metadata:
        // a broad stroke can otherwise fit its mask but leave too little room for the result.
        let tiles = width.div_ceil(TILE) * height.div_ceil(TILE);
        let sparse_max = tiles * (TILE_BYTES + size_of::<u32>()) + height * size_of::<(u32, u32)>();
        let room = pixels::available().saturating_sub(output_bytes);
        let dense_fits_better = cells * size_of::<u16>() <= room && room < sparse_max;
        if cells <= TILE_CELLS * 4 || width < TILE || height < TILE || dense_fits_better {
            let (mut data, lease) = super::reserve_buffer(cells)?;
            data.resize(cells, 0);
            Ok(Self::Dense {
                region,
                cells: data,
                _lease: lease,
                touched: false,
            })
        } else {
            SparseMask::new(region).map(Self::Sparse)
        }
    }
    pub(super) fn get(&self, x: u32, y: u32) -> u16 {
        match self {
            Self::Dense {
                region: [left, top, right, _],
                cells,
                ..
            } => cells[((y - top) * (right - left) + x - left) as usize],
            Self::Sparse(mask) => mask.get(x, y),
        }
    }
    pub(super) fn set(&mut self, x: u32, y: u32, value: u16) -> Result<(), EraseError> {
        match self {
            Self::Dense {
                region: [left, top, right, _],
                cells,
                touched,
                ..
            } => {
                cells[((y - *top) * (*right - *left) + x - *left) as usize] = value;
                *touched = true;
                Ok(())
            }
            Self::Sparse(mask) => mask.set(x, y, value),
        }
    }
    pub(super) fn touched(&self) -> bool {
        match self {
            Self::Dense { touched, .. } => *touched,
            Self::Sparse(mask) => mask.touched(),
        }
    }
    pub(super) fn row_span(&self, y: u32) -> Option<(u32, u32)> {
        match self {
            Self::Dense {
                region: [left, _, right, _],
                ..
            } => Some((*left, *right)),
            Self::Sparse(mask) => mask.row_span(y),
        }
    }
}

pub(super) struct SparseMask {
    left: u32,
    top: u32,
    tiles_x: usize,
    tile_map: Vec<u32>,
    pool: Vec<[u16; TILE_CELLS]>,
    row_spans: Vec<(u32, u32)>,
    lease: Lease,
    touched: bool,
}

impl SparseMask {
    pub(super) fn new(region: [u32; 4]) -> Result<Self, EraseError> {
        let [left, top, right, bottom] = region;
        let width = (right - left) as usize;
        let height = (bottom - top) as usize;
        let tiles_x = width.div_ceil(TILE);
        let tiles_y = height.div_ceil(TILE);
        let map_len = tiles_x
            .checked_mul(tiles_y)
            .ok_or(EraseError::BudgetExceeded)?;
        let meta_bytes = map_len
            .checked_mul(std::mem::size_of::<u32>())
            .and_then(|b| b.checked_add(height.checked_mul(std::mem::size_of::<(u32, u32)>())?))
            .ok_or(EraseError::BudgetExceeded)?;
        let mut lease = pixels::reserve(meta_bytes).ok_or(EraseError::BudgetExceeded)?;
        let mut tile_map = Vec::new();
        tile_map
            .try_reserve_exact(map_len)
            .map_err(|_| EraseError::BudgetExceeded)?;
        let mut row_spans = Vec::new();
        row_spans
            .try_reserve_exact(height)
            .map_err(|_| EraseError::BudgetExceeded)?;
        let actual_meta = tile_map.capacity() * std::mem::size_of::<u32>()
            + row_spans.capacity() * std::mem::size_of::<(u32, u32)>();
        if !lease.grow_to(actual_meta) {
            return Err(EraseError::BudgetExceeded);
        }
        tile_map.resize(map_len, u32::MAX);
        row_spans.resize(height, (u32::MAX, 0));
        Ok(Self {
            left,
            top,
            tiles_x,
            tile_map,
            pool: Vec::new(),
            row_spans,
            lease,
            touched: false,
        })
    }

    #[inline]
    pub(super) fn get(&self, x: u32, y: u32) -> u16 {
        let lx = (x - self.left) as usize;
        let ly = (y - self.top) as usize;
        let slot = self.tile_map[(ly / TILE) * self.tiles_x + (lx / TILE)];
        if slot == u32::MAX {
            0
        } else {
            self.pool[slot as usize][(ly % TILE) * TILE + (lx % TILE)]
        }
    }

    #[inline]
    pub(super) fn set(&mut self, x: u32, y: u32, mask: u16) -> Result<(), EraseError> {
        let lx = (x - self.left) as usize;
        let ly = (y - self.top) as usize;
        let map_idx = (ly / TILE) * self.tiles_x + (lx / TILE);
        let mut slot = self.tile_map[map_idx];
        if slot == u32::MAX {
            if self.pool.len() == self.pool.capacity() {
                let mut grow = (self.pool.len() / 2)
                    .clamp(1, 64)
                    .min(self.tile_map.len() - self.pool.len());
                let meta = self.tile_map.capacity() * std::mem::size_of::<u32>()
                    + self.row_spans.capacity() * std::mem::size_of::<(u32, u32)>();
                // Admit planned capacity before allocating, then account for allocator rounding.
                if !self
                    .lease
                    .grow_to(meta + (self.pool.len() + grow) * TILE_BYTES)
                {
                    grow = 1;
                    if !self
                        .lease
                        .grow_to(meta + (self.pool.len() + 1) * TILE_BYTES)
                    {
                        return Err(EraseError::BudgetExceeded);
                    }
                }
                self.pool
                    .try_reserve_exact(grow)
                    .map_err(|_| EraseError::BudgetExceeded)?;
                let required = meta + self.pool.capacity() * TILE_BYTES;
                if !self.lease.grow_to(required) {
                    return Err(EraseError::BudgetExceeded);
                }
            }
            slot = u32::try_from(self.pool.len()).map_err(|_| EraseError::BudgetExceeded)?;
            self.pool.push([0u16; TILE_CELLS]);
            self.tile_map[map_idx] = slot;
        }
        self.pool[slot as usize][(ly % TILE) * TILE + (lx % TILE)] = mask;
        let span = &mut self.row_spans[ly];
        span.0 = span.0.min(x);
        span.1 = span.1.max(x + 1);
        self.touched = true;
        Ok(())
    }

    pub(super) fn touched(&self) -> bool {
        self.touched
    }

    #[inline]
    pub(super) fn row_span(&self, y: u32) -> Option<(u32, u32)> {
        let (start, end) = self.row_spans[(y - self.top) as usize];
        (start < end).then_some((start, end))
    }
}
