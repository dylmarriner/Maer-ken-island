//! The island's terrain as bytes, and back again.
//!
//! `/api/map.png` is a render: a colour per cell, decided by the server,
//! which is the right answer for a page showing the island and the wrong
//! one for a client that has to put the ground at a height. It cannot
//! recover metres from a colour ramp, and should not try.
//!
//! So the same grid is served as numbers, and the format lives here with
//! the writer and the reader side by side. The one way a binary format
//! goes wrong is that the two ends drift, and the cheapest way to stop
//! that is for there to be only one of each.

use serde::{Deserialize, Serialize};

/// The four bytes every encoding starts with: "Maer-Ken Island
/// Elevation". A reader handed something else has the wrong file and
/// should say so rather than render noise.
pub const MAGIC: [u8; 4] = *b"MKIE";

/// The layout version, bumped if the bytes after the header ever mean
/// something different.
pub const FORMAT: u16 = 1;

/// Bytes of header before the grids begin.
pub const HEADER_BYTES: usize = 4 + 2 + 4 + 4 + 4;

/// Why some bytes are not an island's terrain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainError {
    /// Not this format at all.
    NotTerrain,
    /// This format, from a newer or older server.
    Format { found: u16, expected: u16 },
    /// The right shape of header and the wrong amount of data after it.
    Truncated { expected: usize, found: usize },
}

impl std::fmt::Display for TerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerrainError::NotTerrain => write!(
                f,
                "those bytes are not an island's terrain: they do not start with MKIE"
            ),
            TerrainError::Format { found, expected } => write!(
                f,
                "that terrain is written in format {found} and this reads format {expected}; \
                 the server and this client are different versions"
            ),
            TerrainError::Truncated { expected, found } => write!(
                f,
                "that terrain says it holds {expected} bytes of grid and carries {found}; the \
                 download was cut short"
            ),
        }
    }
}

impl std::error::Error for TerrainError {}

/// The island's ground: a height and a land flag for every medium cell.
///
/// Row 0 is south and column 0 is west, as everywhere else in the domain,
/// and a cell's centre is `((col + 0.5) * cell_size_m, (row + 0.5) *
/// cell_size_m)` — the domain's origin is its own south-west corner, so
/// there is no origin to carry.
///
/// `f32` rather than `f64` halves it and loses nothing that matters: the
/// island's relief is a few thousand metres and `f32` holds that to well
/// under a millimetre. This is for drawing, and it is never hashed — the
/// canonical digest reads the `f64` grid the simulation owns.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Terrain {
    pub rows: usize,
    pub cols: usize,
    pub cell_size_m: f32,
    /// `rows * cols` heights in metres, row-major from the south-west.
    pub elevation_m: Vec<f32>,
    /// `rows * cols` flags; the rest is sea.
    ///
    /// A byte per cell on the wire rather than a bitfield. It compresses
    /// to almost nothing beside the elevations, and a bitfield would make
    /// every client that reads this write a shift and a mask correctly.
    pub land: Vec<bool>,
}

impl Terrain {
    /// Index of a cell in [`Terrain::elevation_m`] and [`Terrain::land`],
    /// or `None` off the grid.
    pub fn index(&self, row: usize, col: usize) -> Option<usize> {
        (row < self.rows && col < self.cols).then(|| row * self.cols + col)
    }

    pub fn elevation_at(&self, row: usize, col: usize) -> Option<f32> {
        self.index(row, col).map(|i| self.elevation_m[i])
    }

    pub fn is_land(&self, row: usize, col: usize) -> Option<bool> {
        self.index(row, col).map(|i| self.land[i])
    }

    /// The centre of a cell in domain metres.
    pub fn centre_m(&self, row: usize, col: usize) -> (f64, f64) {
        let size = f64::from(self.cell_size_m);
        ((col as f64 + 0.5) * size, (row as f64 + 0.5) * size)
    }

    /// The whole domain in metres: how far east and how far north it runs.
    pub fn extent_m(&self) -> (f64, f64) {
        let size = f64::from(self.cell_size_m);
        (self.cols as f64 * size, self.rows as f64 * size)
    }

    /// The cell holding a point, or `None` outside the domain.
    pub fn cell_at_m(&self, x_m: f64, y_m: f64) -> Option<(usize, usize)> {
        let size = f64::from(self.cell_size_m);
        if size <= 0.0 || x_m < 0.0 || y_m < 0.0 {
            return None;
        }
        let col = (x_m / size) as usize;
        let row = (y_m / size) as usize;
        (row < self.rows && col < self.cols).then_some((row, col))
    }

    /// The highest and lowest ground, for a renderer deciding a vertical
    /// scale. `None` on an empty grid.
    pub fn relief_m(&self) -> Option<(f32, f32)> {
        let mut lowest = f32::INFINITY;
        let mut highest = f32::NEG_INFINITY;
        for height in &self.elevation_m {
            lowest = lowest.min(*height);
            highest = highest.max(*height);
        }
        highest.is_finite().then_some((lowest, highest))
    }

    /// Little-endian throughout, because every machine that will read this
    /// is, and saying so is cheaper than a byte-order mark nobody checks.
    ///
    /// | Bytes | Meaning |
    /// |---|---|
    /// | 0..4 | [`MAGIC`] |
    /// | 4..6 | `u16` [`FORMAT`] |
    /// | 6..10 | `u32` rows |
    /// | 10..14 | `u32` cols |
    /// | 14..18 | `f32` cell size, metres |
    /// | 18.. | `rows * cols` `f32` elevations, metres |
    /// | then | `rows * cols` `u8` land mask, 1 for land |
    pub fn encode(&self) -> Vec<u8> {
        let cells = self.rows * self.cols;
        let mut out = Vec::with_capacity(HEADER_BYTES + cells * 5);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&FORMAT.to_le_bytes());
        out.extend_from_slice(&(self.rows as u32).to_le_bytes());
        out.extend_from_slice(&(self.cols as u32).to_le_bytes());
        out.extend_from_slice(&self.cell_size_m.to_le_bytes());
        for height in &self.elevation_m {
            out.extend_from_slice(&height.to_le_bytes());
        }
        for land in &self.land {
            out.push(u8::from(*land));
        }
        out
    }

    /// Read what [`Terrain::encode`] wrote.
    pub fn parse(bytes: &[u8]) -> Result<Self, TerrainError> {
        if bytes.len() < HEADER_BYTES || bytes[..4] != MAGIC {
            return Err(TerrainError::NotTerrain);
        }
        let format = u16::from_le_bytes([bytes[4], bytes[5]]);
        if format != FORMAT {
            return Err(TerrainError::Format {
                found: format,
                expected: FORMAT,
            });
        }
        let rows = u32::from_le_bytes(bytes[6..10].try_into().expect("four bytes")) as usize;
        let cols = u32::from_le_bytes(bytes[10..14].try_into().expect("four bytes")) as usize;
        let cell_size_m = f32::from_le_bytes(bytes[14..18].try_into().expect("four bytes"));
        // Checked before allocating. A header claiming four billion rows
        // would otherwise ask for a reservation nothing can satisfy, on
        // bytes that are plainly not an island.
        // Saturating all the way through. A header claiming four billion
        // rows by four billion columns overflows an ordinary multiply in
        // a debug build and wraps to a small, plausible-looking length in
        // a release one -- which would then pass the check and allocate
        // for seventy quintillion cells. Saturated, it simply cannot
        // match eighteen bytes of input, which is the true answer.
        let cells = rows.saturating_mul(cols);
        let expected = cells.saturating_mul(5).saturating_add(HEADER_BYTES);
        if bytes.len() != expected {
            return Err(TerrainError::Truncated {
                expected,
                found: bytes.len(),
            });
        }
        let mut elevation_m = Vec::with_capacity(cells);
        for i in 0..cells {
            let at = HEADER_BYTES + i * 4;
            elevation_m.push(f32::from_le_bytes(
                bytes[at..at + 4].try_into().expect("four bytes"),
            ));
        }
        let mask = HEADER_BYTES + cells * 4;
        let land = bytes[mask..].iter().map(|byte| *byte == 1).collect();
        Ok(Self {
            rows,
            cols,
            cell_size_m,
            elevation_m,
            land,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> Terrain {
        Terrain {
            rows: 3,
            cols: 2,
            cell_size_m: 2_000.0,
            elevation_m: vec![-10.5, 0.0, 12.25, 300.0, -4000.0, 1.5],
            land: vec![false, true, true, true, false, true],
        }
    }

    #[test]
    fn what_is_written_is_what_is_read() {
        let encoded = small().encode();
        assert_eq!(encoded.len(), HEADER_BYTES + 6 * 5);
        assert_eq!(Terrain::parse(&encoded).unwrap(), small());
    }

    #[test]
    fn the_wrong_bytes_are_refused_by_name() {
        assert_eq!(Terrain::parse(b"").unwrap_err(), TerrainError::NotTerrain);
        assert_eq!(
            Terrain::parse(&[0u8; 64]).unwrap_err(),
            TerrainError::NotTerrain
        );

        let mut newer = small().encode();
        newer[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            Terrain::parse(&newer).unwrap_err(),
            TerrainError::Format {
                found: 99,
                expected: FORMAT
            }
        );

        let encoded = small().encode();
        let cut = &encoded[..encoded.len() - 3];
        assert!(matches!(
            Terrain::parse(cut).unwrap_err(),
            TerrainError::Truncated { .. }
        ));
    }

    #[test]
    fn an_absurd_header_is_refused_rather_than_allocated_for() {
        // Four billion rows by four billion columns. Without the length
        // check first, this reserves for 7e19 cells on eighteen bytes of
        // input.
        let mut absurd = Vec::new();
        absurd.extend_from_slice(&MAGIC);
        absurd.extend_from_slice(&FORMAT.to_le_bytes());
        absurd.extend_from_slice(&u32::MAX.to_le_bytes());
        absurd.extend_from_slice(&u32::MAX.to_le_bytes());
        absurd.extend_from_slice(&1.0f32.to_le_bytes());
        assert!(matches!(
            Terrain::parse(&absurd).unwrap_err(),
            TerrainError::Truncated { .. }
        ));
    }

    #[test]
    fn a_cell_can_be_found_from_metres_and_back() {
        let terrain = small();
        assert_eq!(terrain.centre_m(0, 0), (1_000.0, 1_000.0));
        assert_eq!(terrain.extent_m(), (4_000.0, 6_000.0));
        assert_eq!(terrain.cell_at_m(1_000.0, 1_000.0), Some((0, 0)));
        assert_eq!(terrain.cell_at_m(3_500.0, 5_500.0), Some((2, 1)));
        assert_eq!(terrain.cell_at_m(-1.0, 0.0), None, "west of the domain");
        assert_eq!(terrain.cell_at_m(4_001.0, 0.0), None, "east of it");
        assert_eq!(terrain.elevation_at(2, 1), Some(1.5));
        assert_eq!(terrain.is_land(0, 0), Some(false));
        assert_eq!(terrain.elevation_at(3, 0), None);
        assert_eq!(terrain.relief_m(), Some((-4000.0, 300.0)));
    }
}
