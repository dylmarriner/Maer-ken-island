//! The island's ground, as meshes.
//!
//! One mesh is not an option. The medium grid is 1,200 x 960 cells, which
//! is 1,152,000 quads and over four million vertices in a single buffer --
//! a quarter of a gigabyte that has to be rebuilt whole if anything about
//! it changes, and which no culler can do anything with because it is one
//! object. So the ground is chunked: a grid of fixed-size tiles, each its
//! own mesh, each with its own bounds, so a camera looking at the estate
//! draws the estate.
//!
//! Chunking is deterministic from the grid's shape alone. The same island
//! gives the same chunks in the same order on every machine, which is what
//! lets a test say anything about them at all.
//!
//! Nothing here is Bevy. A chunk is vertices and indices; what turns them
//! into a handle is `render/terrain.rs`, and it is the only part that
//! cannot be tested without a GPU.

use crate::scene::{regional_of, Point};
use mk_island_api::Terrain;

/// Cells along one edge of a chunk.
///
/// Sixty-four gives 4,096 quads -- about 8,450 vertices -- which is a
/// comfortable mesh, and it divides the island's 1,200 x 960 into 19 x 15
/// chunks: 285 draw calls for the whole island, and a handful when the
/// camera is over the estate. Smaller chunks cull better and cost more
/// calls; this is the usual middle.
pub const CHUNK_CELLS: usize = 64;

/// One tile of ground, ready to become a mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// Where this chunk sits in the chunk grid, row then column, from the
    /// south-west. Its identity: the same island gives the same chunk at
    /// the same place every time.
    pub chunk_row: usize,
    pub chunk_col: usize,
    /// The cells it covers, as a half-open range of the terrain grid.
    pub rows: std::ops::Range<usize>,
    pub cols: std::ops::Range<usize>,
    /// The grid rows and columns this chunk actually sampled. At full
    /// detail that is every one in `rows`/`cols`; at a coarser stride it
    /// is every second or fourth, and these are what the vertex grid is
    /// built from.
    pub sampled_rows: Vec<usize>,
    pub sampled_cols: Vec<usize>,
    /// Positions in the regional frame, row-major from the chunk's
    /// south-west corner.
    pub positions: Vec<Point>,
    /// Upward normals, one per position, computed from the neighbouring
    /// heights so a hillside catches light.
    pub normals: Vec<[f32; 3]>,
    /// Triangles, three indices each, into `positions`.
    pub indices: Vec<u32>,
    /// Whether each vertex's cell is land. The renderer picks a material
    /// from this rather than from a height, because the island's sea
    /// level is fitted and "below zero" is not the same question as "is
    /// this sea".
    pub land: Vec<bool>,
    /// The chunk's own extent in the regional frame, for culling and for
    /// a camera that wants to frame it.
    pub min: Point,
    pub max: Point,
}

impl Chunk {
    /// Vertices along each edge. One more than the cells it covers,
    /// because a quad needs both of its corners.
    pub fn vertex_rows(&self) -> usize {
        self.sampled_rows.len()
    }

    pub fn vertex_cols(&self) -> usize {
        self.sampled_cols.len()
    }

    /// Triangles in this chunk, which is what the adapter is asked to
    /// draw and therefore the number worth counting.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// The middle of the chunk, for a camera asked to look at it.
    pub fn centre(&self) -> Point {
        Point::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }
}

/// How many chunks the island divides into.
pub fn chunk_grid(terrain: &Terrain) -> (usize, usize) {
    (
        terrain.rows.div_ceil(CHUNK_CELLS),
        terrain.cols.div_ceil(CHUNK_CELLS),
    )
}

/// Build one chunk, or `None` if it is off the grid.
///
/// Chunks overlap by one row and one column of *vertices*, which is not
/// an accident and not a waste: two chunks that stopped at their own last
/// cell would leave a seam of missing triangles between them, visible as
/// a crack right through the island. Sharing the edge vertices is what
/// closes it.
pub fn chunk(terrain: &Terrain, chunk_row: usize, chunk_col: usize) -> Option<Chunk> {
    chunk_at(terrain, chunk_row, chunk_col, 1)
}

/// How coarsely the ground may be sampled.
///
/// A stride of 1 is every cell; 2 is every second, which is a quarter of
/// the triangles; 4 is a sixteenth. It must divide [`CHUNK_CELLS`], and
/// that is the whole reason this is checked rather than trusted: chunks
/// meet at multiples of 64, so a stride that divides 64 puts a sampled
/// vertex exactly on every shared edge and the chunks still close. A
/// stride of 3 would not, and the island would get a crack through it
/// every 64 cells -- the same defect the shared edge exists to prevent,
/// reintroduced by a number.
pub fn strides() -> [usize; 4] {
    [1, 2, 4, 8]
}

/// Whether this stride keeps the chunks closed.
pub fn stride_is_usable(stride: usize) -> bool {
    stride > 0 && CHUNK_CELLS.is_multiple_of(stride)
}

/// Build one chunk at a given sampling stride.
pub fn chunk_at(
    terrain: &Terrain,
    chunk_row: usize,
    chunk_col: usize,
    stride: usize,
) -> Option<Chunk> {
    if !stride_is_usable(stride) {
        return None;
    }
    let (chunk_rows, chunk_cols) = chunk_grid(terrain);
    if chunk_row >= chunk_rows || chunk_col >= chunk_cols {
        return None;
    }
    let row0 = chunk_row * CHUNK_CELLS;
    let col0 = chunk_col * CHUNK_CELLS;
    // The `+ 1` is the shared edge; `min` keeps the last chunk inside the
    // grid when the island does not divide evenly, which it does not.
    let row1 = (row0 + CHUNK_CELLS + 1).min(terrain.rows);
    let col1 = (col0 + CHUNK_CELLS + 1).min(terrain.cols);
    let (rows, cols) = (row0..row1, col0..col1);
    // The cells actually sampled. At stride 1 this is every one; at 2,
    // every second. The last chunk of the grid is partial and may not
    // land on a stride boundary, so its final row and column are added
    // back -- without them the island would stop short of its own edge.
    let sample = |range: std::ops::Range<usize>| -> Vec<usize> {
        let mut taken: Vec<usize> = range.clone().step_by(stride).collect();
        match (taken.last(), range.end.checked_sub(1)) {
            (Some(&last), Some(edge)) if last != edge => taken.push(edge),
            _ => {}
        }
        taken
    };
    let sampled_rows = sample(rows.clone());
    let sampled_cols = sample(cols.clone());

    let mut positions = Vec::with_capacity(sampled_rows.len() * sampled_cols.len());
    let mut land = Vec::with_capacity(sampled_rows.len() * sampled_cols.len());
    let mut min = Point::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Point::new(f32::MIN, f32::MIN, f32::MIN);

    for &row in &sampled_rows {
        for &col in &sampled_cols {
            let (x_m, y_m) = terrain.centre_m(row, col);
            let elevation = f64::from(terrain.elevation_at(row, col).unwrap_or(0.0));
            let point = regional_of(terrain, x_m, y_m, elevation);
            min = Point::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z));
            max = Point::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z));
            positions.push(point);
            land.push(terrain.is_land(row, col).unwrap_or(false));
        }
    }

    let width = sampled_cols.len();
    let height = sampled_rows.len();
    let mut indices = Vec::with_capacity(height.saturating_sub(1) * width.saturating_sub(1) * 6);
    for r in 0..height.saturating_sub(1) {
        for c in 0..width.saturating_sub(1) {
            let i = (r * width + c) as u32;
            let right = i + 1;
            let up = i + width as u32;
            let up_right = up + 1;
            // Wound so the geometric normal points **up**, which in this
            // frame is not the order that looks right in source. x runs
            // east and z runs *south*, so "counter-clockwise from above"
            // in screen terms is clockwise in the xz-plane, and the
            // obvious `[i, up, right]` gives a normal of -y: every
            // triangle of the island faces the sea floor and is
            // backface-culled from any camera above it.
            //
            // That is exactly what happened. The first frame this
            // application ever drew was a nearly black screen with a few
            // fragments in it, and this was why.
            // `every_triangle_faces_the_sky` is the test that would have
            // caught it, written after the picture did.
            indices.extend_from_slice(&[i, right, up]);
            indices.extend_from_slice(&[right, up_right, up]);
        }
    }

    let normals = normals_for(&positions, height, width);

    Some(Chunk {
        chunk_row,
        chunk_col,
        sampled_rows,
        sampled_cols,
        rows,
        cols,
        positions,
        normals,
        indices,
        land,
        min,
        max,
    })
}

/// Every chunk of the island at full detail, south-west first.
pub fn chunks(terrain: &Terrain) -> Vec<Chunk> {
    chunks_at(terrain, 1)
}

/// Every chunk of the island at a sampling stride.
///
/// The island is 1,200 x 960 cells, which is 2.3 million triangles at
/// full detail -- for a picture about 1,280 pixels across, where the
/// whole island is on screen at once. That is roughly 1,800 triangles
/// per pixel column, and every one of them is transformed whether or not
/// it can be told from its neighbour.
pub fn chunks_at(terrain: &Terrain, stride: usize) -> Vec<Chunk> {
    let (rows, cols) = chunk_grid(terrain);
    (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .filter_map(|(r, c)| chunk_at(terrain, r, c, stride))
        .collect()
}

/// Normals from the heights either side of each vertex.
///
/// The central difference, falling back to a one-sided one at the edges,
/// which is what makes neighbouring chunks agree about the shading along
/// the seam they share. An edge vertex given a flat normal instead would
/// draw a bright line down every chunk boundary.
fn normals_for(positions: &[Point], rows: usize, cols: usize) -> Vec<[f32; 3]> {
    let at = |r: usize, c: usize| positions[r * cols + c];
    let mut normals = Vec::with_capacity(positions.len());
    for r in 0..rows {
        for c in 0..cols {
            let (west, east) = (c.saturating_sub(1), (c + 1).min(cols - 1));
            let (south, north) = (r.saturating_sub(1), (r + 1).min(rows - 1));
            let dx = at(r, east).x - at(r, west).x;
            let dy_x = at(r, east).y - at(r, west).y;
            let dz = at(north, c).z - at(south, c).z;
            let dy_z = at(north, c).y - at(south, c).y;
            // The cross product of the two tangents, with the sign chosen
            // so flat ground points up.
            let normal = [-dy_x * dz, dx * dz, -dx * dy_z];
            let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                .sqrt()
                .max(f32::MIN_POSITIVE);
            let mut unit = [normal[0] / length, normal[1] / length, normal[2] / length];
            if unit[1] < 0.0 {
                unit = [-unit[0], -unit[1], -unit[2]];
            }
            normals.push(unit);
        }
    }
    normals
}

#[cfg(test)]
mod detail_tests {
    use super::*;

    fn a_grid(rows: usize, cols: usize) -> Terrain {
        let mut elevation = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                // A slope with a bump, so decimation has something to lose
                // and the test is not comparing two flat planes.
                elevation.push((r as f32) * 0.5 + (c as f32) * 0.25 + ((r * c) % 7) as f32);
            }
        }
        Terrain {
            rows,
            cols,
            cell_size_m: 2000.0,
            elevation_m: elevation,
            land: vec![true; rows * cols],
        }
    }

    #[test]
    fn a_stride_that_would_crack_the_island_is_refused() {
        // Chunks meet at multiples of 64. A stride of 3 does not divide
        // 64, so the sampled vertices either side of a seam would not
        // line up and the island would show a crack every 64 cells --
        // which is the exact defect the shared edge exists to prevent.
        let terrain = a_grid(130, 130);
        assert!(stride_is_usable(1));
        assert!(stride_is_usable(2));
        assert!(stride_is_usable(8));
        assert!(!stride_is_usable(3));
        assert!(!stride_is_usable(0));
        assert!(!stride_is_usable(7));
        assert!(chunk_at(&terrain, 0, 0, 3).is_none());
        assert!(chunk_at(&terrain, 0, 0, 0).is_none());
    }

    #[test]
    fn neighbouring_chunks_still_share_their_edge_at_every_usable_stride() {
        // The crack test, at each stride. Two chunks side by side must
        // agree exactly on the vertices along the seam -- not nearly,
        // exactly, because a sub-millimetre disagreement is still a hole
        // the sky shows through.
        let terrain = a_grid(200, 200);
        for stride in strides() {
            let left = chunk_at(&terrain, 0, 0, stride).expect("a chunk");
            let right = chunk_at(&terrain, 0, 1, stride).expect("the chunk beside it");
            assert_eq!(
                left.sampled_cols.last(),
                right.sampled_cols.first(),
                "stride {stride}: the chunks do not meet on the same column"
            );
            let width = left.vertex_cols();
            for row in 0..left.vertex_rows() {
                let on_the_left = left.positions[row * width + (width - 1)];
                let on_the_right = right.positions[row * right.vertex_cols()];
                assert_eq!(
                    on_the_left, on_the_right,
                    "stride {stride}, row {row}: the seam does not close"
                );
            }
        }
    }

    #[test]
    fn a_coarser_stride_costs_what_it_should() {
        // Halving the sampling quarters the triangles. Asserted because
        // the whole reason for this knob is the count, and a change that
        // did not reduce it would be all of the quality loss and none of
        // the gain.
        let terrain = a_grid(200, 200);
        let at = |stride| -> usize {
            chunks_at(&terrain, stride)
                .iter()
                .map(|chunk| chunk.triangles())
                .sum()
        };
        let (full, half, quarter) = (at(1), at(2), at(4));
        assert!(full > half && half > quarter, "{full} {half} {quarter}");
        // Not exactly a quarter: chunks share edges, and the last chunk
        // of the grid is partial, so both add vertices the arithmetic
        // does not. Within a fifth of it is the claim worth making.
        let ratio = full as f64 / half as f64;
        assert!(
            (3.2..4.8).contains(&ratio),
            "halving the sampling should quarter the triangles, got {ratio:.2}x"
        );
    }

    #[test]
    fn every_triangle_still_faces_the_sky_at_every_stride() {
        // The defect that made the first frame black, re-checked at each
        // stride: decimation changes which vertices are used, and a
        // winding that survived stride 1 is not thereby proven at 4.
        let terrain = a_grid(140, 140);
        for stride in strides() {
            for chunk in chunks_at(&terrain, stride) {
                for triangle in chunk.indices.chunks(3) {
                    let [a, b, c] = [
                        chunk.positions[triangle[0] as usize],
                        chunk.positions[triangle[1] as usize],
                        chunk.positions[triangle[2] as usize],
                    ];
                    let (u, v) = (
                        (b.x - a.x, b.y - a.y, b.z - a.z),
                        (c.x - a.x, c.y - a.y, c.z - a.z),
                    );
                    let normal_y = u.2 * v.0 - u.0 * v.2;
                    assert!(
                        normal_y > 0.0,
                        "stride {stride}: a triangle faces the sea floor"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(rows: usize, cols: usize) -> Terrain {
        Terrain {
            rows,
            cols,
            cell_size_m: 2_000.0,
            elevation_m: vec![0.0; rows * cols],
            land: vec![true; rows * cols],
        }
    }

    #[test]
    fn the_island_divides_into_chunks_that_cover_every_cell() {
        // 1,200 x 960, the real medium grid.
        let terrain = flat(960, 1_200);
        let (rows, cols) = chunk_grid(&terrain);
        assert_eq!((rows, cols), (15, 19));

        let chunks = chunks(&terrain);
        assert_eq!(chunks.len(), 15 * 19);

        // Every cell is in at least one chunk, and the edge cells are in
        // two -- which is the seam fix, not a bug.
        let mut covered = vec![0u8; terrain.rows * terrain.cols];
        for chunk in &chunks {
            for row in chunk.rows.clone() {
                for col in chunk.cols.clone() {
                    covered[row * terrain.cols + col] += 1;
                }
            }
        }
        assert!(
            covered.iter().all(|n| *n >= 1),
            "{} cells are in no chunk",
            covered.iter().filter(|n| **n == 0).count()
        );
    }

    #[test]
    fn neighbouring_chunks_share_an_edge_rather_than_leaving_a_crack() {
        let terrain = flat(200, 200);
        let a = chunk(&terrain, 0, 0).unwrap();
        let b = chunk(&terrain, 0, 1).unwrap();
        assert_eq!(
            a.cols.end - 1,
            b.cols.start,
            "the chunks do not share a column of vertices, so there is a seam between them"
        );

        // And the shared vertices really are at the same place.
        let a_edge: Vec<Point> = (0..a.rows.len())
            .map(|r| a.positions[r * a.cols.len() + (a.cols.len() - 1)])
            .collect();
        let b_edge: Vec<Point> = (0..b.rows.len())
            .map(|r| b.positions[r * b.cols.len()])
            .collect();
        assert_eq!(a_edge, b_edge);
    }

    #[test]
    fn a_chunk_is_a_mesh_a_renderer_can_take() {
        let terrain = flat(200, 200);
        let chunk = chunk(&terrain, 1, 1).unwrap();
        assert_eq!(chunk.positions.len(), chunk.normals.len());
        assert_eq!(chunk.positions.len(), chunk.land.len());
        assert_eq!(chunk.indices.len() % 3, 0);
        assert!(
            chunk
                .indices
                .iter()
                .all(|i| (*i as usize) < chunk.positions.len()),
            "an index points past the end of the vertices"
        );
        // 65 x 65 vertices, 64 x 64 quads, two triangles each.
        assert_eq!(chunk.positions.len(), 65 * 65);
        assert_eq!(chunk.indices.len(), 64 * 64 * 6);
    }

    /// The normal a rasteriser computes from the vertex order, which is
    /// the one that decides whether a triangle is drawn at all. Not the
    /// shading normal in `Chunk::normals` -- those were right while every
    /// triangle was still facing downward.
    fn geometric_normal(a: Point, b: Point, c: Point) -> [f32; 3] {
        let (u, v) = (
            [b.x - a.x, b.y - a.y, b.z - a.z],
            [c.x - a.x, c.y - a.y, c.z - a.z],
        );
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    #[test]
    fn every_triangle_faces_the_sky() {
        // Backface culling throws away triangles whose geometric normal
        // points away from the camera, and a camera looking at this
        // island is above it. Wound the other way, every one of these is
        // discarded and the island renders as nothing at all -- which is
        // what the first frame this application drew actually looked
        // like.
        //
        // This is about vertex *order*, not about the normals in
        // `Chunk::normals`: those were already correct, and did not save
        // it.
        let terrain = flat(80, 80);
        let chunk = chunk(&terrain, 0, 0).unwrap();
        for (n, tri) in chunk.indices.chunks_exact(3).enumerate() {
            let normal = geometric_normal(
                chunk.positions[tri[0] as usize],
                chunk.positions[tri[1] as usize],
                chunk.positions[tri[2] as usize],
            );
            assert!(
                normal[1] > 0.0,
                "triangle {n} ({tri:?}) faces downward: geometric normal {normal:?}"
            );
        }
    }

    #[test]
    fn flat_ground_points_straight_up() {
        let terrain = flat(80, 80);
        let chunk = chunk(&terrain, 0, 0).unwrap();
        for (i, normal) in chunk.normals.iter().enumerate() {
            assert!(
                (normal[1] - 1.0).abs() < 1e-5,
                "vertex {i} on flat ground has normal {normal:?}"
            );
        }
    }

    #[test]
    fn a_slope_leans_the_way_it_falls() {
        // Ground rising to the east: the normal must tilt west, away from
        // the rise. A sign error here lights every hillside from the
        // wrong side, which looks like a texture problem and is not.
        let rows = 70;
        let cols = 70;
        let mut terrain = flat(rows, cols);
        for row in 0..rows {
            for col in 0..cols {
                terrain.elevation_m[row * cols + col] = col as f32 * 100.0;
            }
        }
        let chunk = chunk(&terrain, 0, 0).unwrap();
        let middle =
            chunk.normals[(chunk.rows.len() / 2) * chunk.cols.len() + chunk.cols.len() / 2];
        assert!(
            middle[0] < 0.0,
            "the slope leans east, not west: {middle:?}"
        );
        assert!(middle[1] > 0.0, "a normal must point upward: {middle:?}");
        assert!(
            middle[2].abs() < 1e-5,
            "there is no north-south slope: {middle:?}"
        );
    }

    #[test]
    fn sea_is_marked_as_sea_rather_than_guessed_from_a_height() {
        // The island's sea level is fitted to a target land area, so
        // "below zero" and "is sea" are different questions. A renderer
        // that guessed would paint a below-sea-level plain as ocean.
        let mut terrain = flat(70, 70);
        terrain.elevation_m[0] = -50.0;
        terrain.land[0] = true;
        terrain.elevation_m[1] = 20.0;
        terrain.land[1] = false;
        let chunk = chunk(&terrain, 0, 0).unwrap();
        assert!(chunk.land[0], "land below sea level is still land");
        assert!(!chunk.land[1], "sea above zero is still sea");
    }

    #[test]
    fn chunking_is_the_same_on_every_machine() {
        let terrain = flat(200, 200);
        assert_eq!(chunks(&terrain), chunks(&terrain));
        assert_eq!(chunk(&terrain, 2, 1), chunk(&terrain, 2, 1));
        assert_eq!(chunk(&terrain, 99, 0), None);
    }

    #[test]
    fn a_chunk_knows_its_own_extent() {
        let terrain = flat(200, 200);
        let chunk = chunk(&terrain, 0, 0).unwrap();
        for point in &chunk.positions {
            assert!(point.x >= chunk.min.x && point.x <= chunk.max.x);
            assert!(point.z >= chunk.min.z && point.z <= chunk.max.z);
        }
        let centre = chunk.centre();
        assert!(centre.x > chunk.min.x && centre.x < chunk.max.x);
    }
}
