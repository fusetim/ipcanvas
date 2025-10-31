//! Canvas-related functionality and operations

pub mod diff;

/// Color of a pixel on the canvas.
///
/// Simple RGB representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A pixel on the canvas with its coordinates and color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pixel {
    pub x: u16,
    pub y: u16,
    pub color: PixelColor,
}

pub mod colors {
    use super::PixelColor;

    /// Predefined colors
    pub const WHITE: PixelColor = PixelColor {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLACK: PixelColor = PixelColor { r: 0, g: 0, b: 0 };
    pub const RED: PixelColor = PixelColor { r: 255, g: 0, b: 0 };
    pub const GREEN: PixelColor = PixelColor { r: 0, g: 255, b: 0 };
    pub const BLUE: PixelColor = PixelColor { r: 0, g: 0, b: 255 };
    pub const YELLOW: PixelColor = PixelColor {
        r: 255,
        g: 255,
        b: 0,
    };
    pub const CYAN: PixelColor = PixelColor {
        r: 0,
        g: 255,
        b: 255,
    };
    pub const MAGENTA: PixelColor = PixelColor {
        r: 255,
        g: 0,
        b: 255,
    };
}

/// Canvas state
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
    width: u16,
    height: u16,
    // Pixel data stored as a flat array.
    // Cell (x, y) is at index (y * width + x)
    data: Box<[PixelColor]>,
}

impl Canvas {
    /// Create a new canvas with the given width and height.
    pub fn new(width: u16, height: u16) -> Self {
        let data = vec![colors::WHITE; (width as usize) * (height as usize)].into_boxed_slice();
        Self {
            width,
            height,
            data,
        }
    }

    /// Get the pixel color at the given coordinates.
    pub fn get_pixel(&self, x: u16, y: u16) -> Option<PixelColor> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        Some(self.data[index])
    }

    /// Set the pixel color at the given coordinates.
    ///
    /// Returns Err(()) if the coordinates are out of bounds.
    pub fn set_pixel(&mut self, x: u16, y: u16, color: PixelColor) -> Result<(), ()> {
        if x >= self.width || y >= self.height {
            return Err(());
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        self.data[index] = color;
        Ok(())
    }

    /// Get the width of the canvas.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Get the height of the canvas.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get an iterator over all pixels in the canvas.
    pub fn pixels<'a>(&'a self) -> CanvasPixelIter<'a> {
        CanvasPixelIter::new(self)
    }
}

impl<'a> IntoIterator for &'a Canvas {
    type Item = Pixel;
    type IntoIter = CanvasPixelIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CanvasPixelIter::new(self)
    }
}

pub struct CanvasPixelIter<'a> {
    canvas: &'a Canvas,
    x: u16,
    y: u16,
}

impl<'a> CanvasPixelIter<'a> {
    /// Create a new iterator over the pixels of the given canvas.
    fn new(canvas: &'a Canvas) -> Self {
        Self { canvas, x: 0, y: 0 }
    }
}

impl<'a> Iterator for CanvasPixelIter<'a> {
    type Item = Pixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y >= self.canvas.height {
            return None;
        }

        let pixel = Pixel {
            x: self.x,
            y: self.y,
            color: self.canvas.get_pixel(self.x, self.y)?,
        };

        self.x += 1;
        if self.x >= self.canvas.width {
            self.x = 0;
            self.y += 1;
        }

        Some(pixel)
    }
}

impl<'a> ExactSizeIterator for CanvasPixelIter<'a> {
    fn len(&self) -> usize {
        (self.canvas.width as usize * self.canvas.height as usize)
            - (self.y as usize * self.canvas.width as usize + self.x as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_pixel_set_get() {
        let mut canvas = Canvas::new(10, 10);
        let red = PixelColor { r: 255, g: 0, b: 0 };
        canvas.set_pixel(5, 5, red).unwrap();
        let color = canvas.get_pixel(5, 5).unwrap();
        assert_eq!(color, red);
    }

    #[test]
    fn test_canvas_pixel_big_canvas_set_get() {
        let mut canvas = Canvas::new(4096, 4096);
        let red = PixelColor { r: 255, g: 0, b: 0 };
        canvas.set_pixel(5, 5, red).unwrap();
        let color = canvas.get_pixel(5, 5).unwrap();
        assert_eq!(color, red);
    }

    #[test]
    fn test_canvas_pixel_get_out_of_bounds() {
        let mut canvas = Canvas::new(10, 10);
        let red = PixelColor { r: 255, g: 0, b: 0 };
        canvas.set_pixel(5, 5, red).unwrap();
        let color = canvas.get_pixel(5, 5).unwrap();
        assert_eq!(color, red);
        assert!(canvas.get_pixel(10, 10).is_none());
    }

    #[test]
    fn test_canvas_pixel_iter() {
        let mut canvas = Canvas::new(2, 2);
        let red = PixelColor { r: 255, g: 0, b: 0 };
        let green = PixelColor { r: 0, g: 255, b: 0 };
        let blue = PixelColor { r: 0, g: 0, b: 255 };
        let white = PixelColor {
            r: 255,
            g: 255,
            b: 255,
        };
        canvas.set_pixel(0, 0, red).unwrap();
        canvas.set_pixel(1, 0, green).unwrap();
        canvas.set_pixel(0, 1, blue).unwrap();
        // (1,1) remains white

        let pixels: Vec<Pixel> = canvas.pixels().collect();
        assert_eq!(pixels.len(), 4);
        assert_eq!(
            pixels[0],
            Pixel {
                x: 0,
                y: 0,
                color: red
            }
        );
        assert_eq!(
            pixels[1],
            Pixel {
                x: 1,
                y: 0,
                color: green
            }
        );
        assert_eq!(
            pixels[2],
            Pixel {
                x: 0,
                y: 1,
                color: blue
            }
        );
        assert_eq!(
            pixels[3],
            Pixel {
                x: 1,
                y: 1,
                color: white
            }
        );
    }
}
