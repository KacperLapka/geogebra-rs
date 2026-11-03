use lyon_tessellation::math::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::*;

const PROBING_SIZE: usize = 1000;
const STEP: f32 = 2.0 / PROBING_SIZE as f32;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MyVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub fn points_raw_to_path(points: &[f32; PROBING_SIZE]) -> Path {
    let mut builder = Path::builder();

    let mut points_iter = points.iter().enumerate();
    let starting_point = point(-1.0, *points_iter.next().unwrap_or((0, &0.0)).1);
    builder.begin(starting_point);

    for (i, y) in points_iter {
        let x = -1.0 + (i as f32 * STEP);
        builder.line_to(point(x, *y));
    }

    builder.end(false);
    builder.build()
}

pub fn path_to_vertex(path: Path) -> VertexBuffers<MyVertex, u16> {
    let mut geometry: VertexBuffers<MyVertex, u16> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();
    {
        // Compute the tessellation.
        tessellator
            .tessellate_path(
                &path,
                &StrokeOptions::default().with_line_width(0.008),
                &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                    MyVertex {
                        position: [vertex.position().x, vertex.position().y, 0.0],
                        color: [1.0, 0.0, 0.0], // Red color
                    }
                }),
            )
            .unwrap();
    }
    // The tessellated geometry is ready to be uploaded to the GPU.
    return geometry;
}

pub fn example() -> VertexBuffers<MyVertex, u16> {
    let mut builder = Path::builder();
    builder.begin(point(-1.0, 0.0));

    // Generujemy punkty funkcji (np. wykres y = sin(x) w skali)
    for i in 1..=100 {
        let x = -1.0 + (i as f32 / 100.0) * 2.0; // Od -1.0 do 1.0
        let y = (x * 3.14).sin() * 0.5; // Jakaś funkcja matematyczna

        // Dodajemy kolejny odcinek
        builder.line_to(point(x, y));
    }
    builder.end(false);
    let path = builder.build();

    let mut geometry: VertexBuffers<MyVertex, u16> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();
    {
        // Compute the tessellation.
        tessellator
            .tessellate_path(
                &path,
                &StrokeOptions::default().with_line_width(0.008),
                &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                    MyVertex {
                        position: [vertex.position().x, vertex.position().y, 0.0],
                        color: [1.0, 0.0, 0.0], // Red color
                    }
                }),
            )
            .unwrap();
    }
    // The tessellated geometry is ready to be uploaded to the GPU.
    return geometry;
}
