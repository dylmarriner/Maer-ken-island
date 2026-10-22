//! The island's standing vegetation as bytes, and back again.
//!
//! `/api/vegetation.png` is a picture of the biomass: a colour per cell,
//! decided by the server. A client that tints its own ground, or decides
//! where a stand of trees goes beyond the individually-modelled patch,
//! needs the number, not a colour it would have to read a number back out
//! of. So the biomass grid is served as numbers too, beside the terrain
//! (`terrain.rs`) and on the same terms: the writer and the reader live
//! here together so the two ends cannot drift.
//!
//! Unlike the terrain this moves. Biomass grows and is cut, so the server
//! redraws it on the stems' cadence and a client reads it again when it
//! wants the forest as it is now.

use serde::{Deserialize, Serialize};

/// The four bytes every encoding starts with: "Maer-Ken Island Cover".
pub const MAGIC: [u8; 4] = *b"MKIC";

/// The layout version, bumped if the bytes after the header ever mean
/// something different.
pub const FORMAT: u16 = 1;

/// Bytes of header before the grid begins.
pub const HEADER_BYTES: usize = 4 + 2 + 4 + 4;

/// Why some bytes are not an island's cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverError {
    /// Not this format at all.
    NotCover,
    /// This format, from a newer or older server.
    Format { found: u16, expected: u16 },
    /// The right shape of header and the wrong amount of data after it.
    Truncated { expected: usize, found: usize },
}

impl std::fmt::Display for CoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverError::NotCover => write!(
                f,
                "those bytes are not an island's vegetation cover: they do not start with MKIC"
            ),
            CoverError::Format { found, expected } => write!(
                f,
                "that cover is written in format {found} and this reads format {expected}; \
                 the server and this client are different versions"
            ),
            CoverError::Truncated { expected, found } => write!(
                f,
                "that cover says it holds {expected} bytes of grid and carries {found}; the \
                 download was cut short"
            ),
        }
    }
}

impl std::error::Error for CoverError {}

/// The standing biomass of every medium cell, kg of carbon per m².
///
/// The same grid as [`crate::Terrain`]: row 0 is south, column 0 is west,
/// row-major. `f32` for the same reason as the terrain's heights: this is
/// for drawing and is never hashed.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Cover {
    pub rows: usize,
    pub cols: usize,
    /// `rows * cols` values, kg C/m²; zero on the sea and bare ground.
    pub biomass_kgc_m2: Vec<f32>,
}

impl Cover {
    /// A cell's biomass, or `None` off the grid.
    pub fn at(&self, row: usize, col: usize) -> Option<f32> {
        (row < self.rows && col < self.cols).then(|| self.biomass_kgc_m2[row * self.cols + col])
    }

    /// The most biomass any cell holds, for a renderer scaling a tint to
    /// this island rather than to a number chosen elsewhere. Zero on an
    /// island with no vegetation, never NaN.
    pub fn peak_kgc_m2(&self) -> f32 {
        self.biomass_kgc_m2
            .iter()
            .copied()
            .filter(|b| b.is_finite())
            .fold(0.0, f32::max)
    }

    /// How fully a cell is stocked, 0 to 1: its biomass over the island's
    /// densest cell. `None` off the grid; 0 everywhere when nothing grows.
    pub fn fraction_at(&self, row: usize, col: usize, peak_kgc_m2: f32) -> Option<f32> {
        let biomass = self.at(row, col)?;
        Some(if peak_kgc_m2 > 0.0 && biomass.is_finite() {
            (biomass / peak_kgc_m2).clamp(0.0, 1.0)
        } else {
            0.0
        })
    }

    /// Little-endian throughout, as the terrain is.
    ///
    /// | Bytes | Meaning |
    /// |---|---|
    /// | 0..4 | [`MAGIC`] |
    /// | 4..6 | `u16` [`FORMAT`] |
    /// | 6..10 | `u32` rows |
    /// | 10..14 | `u32` cols |
    /// | 14.. | `rows * cols` `f32` biomass, kg C/m² |
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.biomass_kgc_m2.len() * 4);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&FORMAT.to_le_bytes());
        out.extend_from_slice(&(self.rows as u32).to_le_bytes());
        out.extend_from_slice(&(self.cols as u32).to_le_bytes());
        for biomass in &self.biomass_kgc_m2 {
            out.extend_from_slice(&biomass.to_le_bytes());
        }
        out
    }

    /// Read what [`Cover::encode`] wrote.
    pub fn parse(bytes: &[u8]) -> Result<Self, CoverError> {
        if bytes.len() < HEADER_BYTES || bytes[..4] != MAGIC {
            return Err(CoverError::NotCover);
        }
        let format = u16::from_le_bytes([bytes[4], bytes[5]]);
        if format != FORMAT {
            return Err(CoverError::Format {
                found: format,
                expected: FORMAT,
            });
        }
        let rows = u32::from_le_bytes(bytes[6..10].try_into().expect("four bytes")) as usize;
        let cols = u32::from_le_bytes(bytes[10..14].try_into().expect("four bytes")) as usize;
        // Saturating, and checked before allocating, for the reason
        // `Terrain::parse` gives: a header claiming billions of cells must
        // fail the length check, not wrap past it.
        let cells = rows.saturating_mul(cols);
        let expected = cells.saturating_mul(4).saturating_add(HEADER_BYTES);
        if bytes.len() != expected {
            return Err(CoverError::Truncated {
                expected,
                found: bytes.len(),
            });
        }
        let biomass_kgc_m2 = bytes[HEADER_BYTES..]
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("four bytes")))
            .collect();
        Ok(Self {
            rows,
            cols,
            biomass_kgc_m2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_cover() -> Cover {
        Cover {
            rows: 2,
            cols: 3,
            biomass_kgc_m2: vec![0.0, 1.5, 3.0, 6.0, 0.25, 12.0],
        }
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let cover = a_cover();
        let bytes = cover.encode();
        assert_eq!(bytes.len(), HEADER_BYTES + 6 * 4);
        assert_eq!(Cover::parse(&bytes), Ok(cover));
    }

    #[test]
    fn a_cell_is_found_by_row_from_the_south_and_column_from_the_west() {
        let cover = a_cover();
        assert_eq!(cover.at(0, 1), Some(1.5));
        assert_eq!(cover.at(1, 2), Some(12.0));
        assert_eq!(cover.at(2, 0), None);
        assert_eq!(cover.at(0, 3), None);
    }

    #[test]
    fn a_cell_is_stocked_against_the_islands_densest() {
        let cover = a_cover();
        let peak = cover.peak_kgc_m2();
        assert_eq!(peak, 12.0);
        assert_eq!(cover.fraction_at(1, 0, peak), Some(0.5));
        assert_eq!(cover.fraction_at(1, 2, peak), Some(1.0));
        assert_eq!(cover.fraction_at(0, 0, peak), Some(0.0));
        let bare = Cover {
            rows: 1,
            cols: 1,
            biomass_kgc_m2: vec![0.0],
        };
        assert_eq!(bare.fraction_at(0, 0, bare.peak_kgc_m2()), Some(0.0));
    }

    #[test]
    fn bytes_that_are_not_a_cover_say_so() {
        assert_eq!(Cover::parse(b"MKIE...."), Err(CoverError::NotCover));
        let mut bytes = a_cover().encode();
        bytes[4] = 9;
        assert_eq!(
            Cover::parse(&bytes),
            Err(CoverError::Format {
                found: 9,
                expected: FORMAT
            })
        );
        let mut short = a_cover().encode();
        short.pop();
        assert!(matches!(
            Cover::parse(&short),
            Err(CoverError::Truncated { .. })
        ));
    }

    #[test]
    fn a_header_claiming_billions_of_cells_is_refused_without_allocating() {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend_from_slice(&FORMAT.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Cover::parse(&bytes),
            Err(CoverError::Truncated { .. })
        ));
    }
}
