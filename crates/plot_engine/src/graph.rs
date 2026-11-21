//! Pure geometry for the plotted curve.
//!
//! `math_engine` returns 1000 y samples already normalised to `[-1, 1]`. This
//! module turns those samples into a lyon path and tessellates it into vertices
//! that the GPU can draw as a triangle list.

use lyon_tessellation::math::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::pipelines::Vertex;

/// Number of samples produced by `math_engine` for each function.
pub const CURVE_SAMPLE_COUNT: usize = 1000;
/// Clip-space x coordinate of the first sample.
const CURVE_X_START: f32 = -1.0;
/// Horizontal distance between consecutive samples in clip space.
const CURVE_X_STEP: f32 = 2.0 / CURVE_SAMPLE_COUNT as f32;
/// Stroke width of the curve in clip-space units.
const CURVE_WIDTH: f32 = 0.008;
/// Colour applied to every curve vertex.
const CURVE_COLOR: [f32; 3] = [1.0, 0.0, 0.0];

/// Converts normalised y samples into a polyline spanning clip space.
///
/// Returns `None` when there are no samples, which lets the renderer keep an
/// empty scene instead of uploading zero-length buffers.
pub fn points_raw_to_path(points: &[f32]) -> Option<Path> {
    // `split_first` is the explicit empty-input guard: no samples, no path.
    let (first, rest) = points.split_first()?;

    let mut builder = Path::builder();
    builder.begin(point(CURVE_X_START, *first));

    for (index, y) in rest.iter().enumerate() {
        // The first sample is at index 0, so `rest` starts at index 1.
        let x = CURVE_X_START + (index + 1) as f32 * CURVE_X_STEP;
        builder.line_to(point(x, *y));
    }

    builder.end(false);
    Some(builder.build())
}

/// Tessellates a curve path into a triangle list with the plot's stroke style.
pub fn tessellate_curve(path: &Path) -> VertexBuffers<Vertex, u16> {
    let mut geometry = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();

    tessellator
        .tessellate_path(
            path,
            &StrokeOptions::default().with_line_width(CURVE_WIDTH),
            &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| Vertex {
                position: [vertex.position().x, vertex.position().y, 0.0],
                color: CURVE_COLOR,
            }),
        )
        .expect("curve tessellation should not fail for a valid path");

    geometry
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyon_tessellation::path::PathEvent;

    #[test]
    fn empty_samples_produce_no_path() {
        assert!(points_raw_to_path(&[]).is_none());
    }

    #[test]
    fn samples_map_across_clip_space() {
        let path = points_raw_to_path(&[0.0, 0.5, -0.5]).unwrap();
        let mut lines = path.iter().filter_map(|event| match event {
            PathEvent::Line { from, to } => Some((from, to)),
            _ => None,
        });

        let (first_from, first_to) = lines.next().unwrap();
        assert_eq!(first_from, point(CURVE_X_START, 0.0));
        assert_eq!(first_to, point(CURVE_X_START + CURVE_X_STEP, 0.5));

        let (second_from, second_to) = lines.next().unwrap();
        assert_eq!(second_from, first_to);
        assert_eq!(second_to, point(CURVE_X_START + 2.0 * CURVE_X_STEP, -0.5));

        assert!(lines.next().is_none());
    }

    #[test]
    fn tessellated_curve_uses_plot_style() {
        let path = points_raw_to_path(&[0.0, 0.5, 0.0, -0.5]).unwrap();
        let geometry = tessellate_curve(&path);

        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.indices.is_empty());
        for vertex in &geometry.vertices {
            assert_eq!(vertex.color, CURVE_COLOR);
            assert_eq!(vertex.position[2], 0.0);
        }
    }

    #[test]
    fn single_sample_tessellates_to_empty_geometry() {
        let path = points_raw_to_path(&[0.5]).unwrap();
        let geometry = tessellate_curve(&path);

        assert!(geometry.vertices.is_empty());
        assert!(geometry.indices.is_empty());
    }
}
