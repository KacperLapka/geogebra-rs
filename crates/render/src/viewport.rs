//! Viewport: math-coordinate window owned by the session (AD-4, AD-5).
//!
//! Svelte sets it via `update_viewport` and never holds its own copy.

/// Visible math-coordinate rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x_min: -10.0,
            x_max: 10.0,
            y_min: -10.0,
            y_max: 10.0,
        }
    }
}

impl Viewport {
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    pub fn width(&self) -> f64 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> f64 {
        self.y_max - self.y_min
    }

    /// Math point -> pixel coordinates in a `w x h` canvas.
    pub fn world_to_screen(&self, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
        let px = (x - self.x_min) / self.width() * w;
        let py = (self.y_max - y) / self.height() * h;
        (px, py)
    }

    /// Pixel coordinates in a `w x h` canvas -> math point.
    pub fn screen_to_world(&self, px: f64, py: f64, w: f64, h: f64) -> (f64, f64) {
        let x = self.x_min + px / w * self.width();
        let y = self.y_max - py / h * self.height();
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_coordinates() {
        let v = Viewport::default();
        let (px, py) = v.world_to_screen(2.0, 3.0, 800.0, 600.0);
        let (x, y) = v.screen_to_world(px, py, 800.0, 600.0);
        assert!((x - 2.0).abs() < 1e-9);
        assert!((y - 3.0).abs() < 1e-9);
    }
}
