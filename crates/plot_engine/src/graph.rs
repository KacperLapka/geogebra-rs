//! Pure geometry for the plotted curve.
//!
//! `math_engine` produces exactly [`CURVE_SAMPLE_COUNT`] (1000) y samples per
//! function. Each sample is mapped from the fixed `[-10, 10]` math window into
//! clip space, so y values outside that window can fall outside `[-1, 1]` and
//! are not clamped here. Sample x coordinates span `-1.0` to `0.998`, stepping
//! by `2.0 / 1000`. This module turns those samples into a lyon path and
//! tessellates it into vertices that the GPU can draw as a triangle list.

use lyon_tessellation::math::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::pipelines::Vertex;

/// Number of samples produced by `math_engine`; the path contract expects this
/// exact count.
pub const CURVE_SAMPLE_COUNT: usize = 1000;
/// Clip-space x coordinate of the first sample.
const CURVE_X_START: f32 = -1.0;
/// Horizontal distance between consecutive samples in clip space.
const CURVE_X_STEP: f32 = 2.0 / CURVE_SAMPLE_COUNT as f32;
/// Stroke width of the curve in clip-space units.
const CURVE_WIDTH: f32 = 0.008;
/// Colour applied to every curve vertex.
const CURVE_COLOR: [f32; 3] = [1.0, 0.0, 0.0];

/// Converts y samples into a polyline spanning clip space.
///
/// The x coordinates run from `-1.0` to `0.998` for the full 1000-sample set.
/// Returns `None` when the input is empty or contains a non-finite y value,
/// letting the renderer keep an empty scene instead of uploading invalid
/// geometry.
pub fn points_raw_to_path(points: &[f32]) -> Option<Path> {
    if points.is_empty() || points.iter().any(|y| !y.is_finite()) {
        return None;
    }

    debug_assert_eq!(
        points.len(),
        CURVE_SAMPLE_COUNT,
        "expected {} curve samples, got {}",
        CURVE_SAMPLE_COUNT,
        points.len()
    );

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

    fn flat_samples() -> Vec<f32> {
        vec![0.0; CURVE_SAMPLE_COUNT]
    }

    #[test]
    fn empty_samples_produce_no_path() {
        assert!(points_raw_to_path(&[]).is_none());
    }

    #[test]
    fn non_finite_samples_produce_no_path() {
        let mut samples = flat_samples();
        samples[0] = f32::INFINITY;
        assert!(points_raw_to_path(&samples).is_none());

        samples[0] = f32::NAN;
        assert!(points_raw_to_path(&samples).is_none());
    }

    #[test]
    fn samples_map_across_clip_space() {
        let mut samples = flat_samples();
        samples[1] = 0.5;
        samples[2] = -0.5;
        samples[CURVE_SAMPLE_COUNT - 1] = 0.25;

        let path = points_raw_to_path(&samples).unwrap();
        let mut lines = path.iter().filter_map(|event| match event {
            PathEvent::Line { from, to } => Some((from, to)),
            _ => None,
        });

        let (first_from, first_to) = lines.next().unwrap();
        assert_eq!(first_from, point(-1.0, 0.0));
        assert_eq!(first_to, point(-0.998, 0.5));

        let (second_from, second_to) = lines.next().unwrap();
        assert_eq!(second_from, first_to);
        assert_eq!(second_to, point(-0.996, -0.5));

        let (_, last_to) = lines.last().unwrap();
        assert!((last_to.x - 0.998).abs() < 1e-5, "last x was {}", last_to.x);
        assert_eq!(last_to.y, 0.25);
    }

    #[test]
    fn tessellated_curve_uses_plot_style_and_width() {
        let path = points_raw_to_path(&flat_samples()).unwrap();
        let geometry = tessellate_curve(&path);

        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.indices.is_empty());
        for vertex in &geometry.vertices {
            assert_eq!(vertex.color, CURVE_COLOR);
            assert_eq!(vertex.position[2], 0.0);
        }

        let min_y = geometry
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = geometry
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let stroke_height = max_y - min_y;

        assert!(
            (stroke_height - 0.008).abs() < 0.001,
            "expected the 0.008 curve stroke, got height {stroke_height}"
        );
    }

    #[test]
    fn single_point_path_tessellates_to_empty_geometry() {
        let mut builder = Path::builder();
        builder.begin(point(0.5, 0.0));
        builder.end(false);

        let geometry = tessellate_curve(&builder.build());

        assert!(geometry.vertices.is_empty());
        assert!(geometry.indices.is_empty());
    }
}
