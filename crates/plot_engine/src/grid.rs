//! Static grid geometry.

use crate::pipelines::Vertex;

/// Number of grid intervals on each side of the origin.
const GRID_HALF_INTERVALS: i32 = 10;
/// Distance between adjacent grid lines in clip space.
const GRID_LINE_SPACING: f32 = 0.1;
/// Clip-space coordinate of the grid's outer edge.
const GRID_EXTENT: f32 = 1.0;
/// Colour applied to every grid vertex.
const GRID_COLOR: [f32; 3] = [0.5, 0.5, 0.5];

/// Builds the grid as vertical and horizontal line segments spanning clip space.
///
/// The result is used with a `LineList` pipeline and never changes at runtime.
pub fn build_grid_vertices() -> Vec<Vertex> {
    let line_count = (GRID_HALF_INTERVALS * 2 + 1) as usize;
    let mut vertices = Vec::with_capacity(line_count * 4);

    for index in -GRID_HALF_INTERVALS..=GRID_HALF_INTERVALS {
        let offset = index as f32 * GRID_LINE_SPACING;
        for position in [
            [offset, -GRID_EXTENT, 0.0],
            [offset, GRID_EXTENT, 0.0],
            [-GRID_EXTENT, offset, 0.0],
            [GRID_EXTENT, offset, 0.0],
        ] {
            vertices.push(Vertex {
                position,
                color: GRID_COLOR,
            });
        }
    }

    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_spans_clip_space() {
        let vertices = build_grid_vertices();
        let line_count = (GRID_HALF_INTERVALS * 2 + 1) as usize;

        assert_eq!(vertices.len(), line_count * 4);
        assert!(vertices.iter().all(|vertex| vertex.color == GRID_COLOR));
        assert!(vertices.iter().all(|vertex| {
            vertex.position[0].abs() <= GRID_EXTENT && vertex.position[1].abs() <= GRID_EXTENT
        }));
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.position[0] == GRID_EXTENT)
        );
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.position[1] == -GRID_EXTENT)
        );
    }
}
