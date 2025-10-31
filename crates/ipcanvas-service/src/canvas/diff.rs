use crate::canvas::{Canvas, Pixel};

/// Represents the difference between two canvas states.
pub struct CanvasDiff {
    pub(crate) changed_pixels: Vec<Pixel>,
}

impl CanvasDiff {
    /// Create a new, empty CanvasDiff.
    pub fn new() -> Self {
        Self {
            changed_pixels: Vec::new(),
        }
    }

    /// Get an iterator over the changed pixels.
    pub fn changed_pixels(&self) -> impl Iterator<Item = &Pixel> + ExactSizeIterator{
        self.changed_pixels.iter()
    }

    /// Check if there are any changes in the diff.
    pub fn is_empty(&self) -> bool {
        self.changed_pixels.is_empty()
    }
}

impl Canvas {
    /// Calculate the diff between this canvas and another canvas.
    pub fn diff(&self, other: &Canvas) -> CanvasDiff {
        let mut diff = CanvasDiff::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let color_self = self.get_pixel(x, y);
                let color_other = other.get_pixel(x, y);

                if color_self != color_other {
                    if let Some(color) = color_other {
                        diff.changed_pixels.push(Pixel { x, y, color });
                    }
                }
            }
        }

        diff
    }
}