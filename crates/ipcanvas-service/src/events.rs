use crate::canvas::PixelColor;

/// Events that can be performed on the canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// Place a pixel at the specified coordinates with the given color.
    PlacePixel { x: u16, y: u16, color: PixelColor },
    /// Place a label at the specified coordinates with the given text.
    ///
    /// The text is limited to 8 bytes. If the text is shorter than 8 bytes,
    /// it should be null-padded.
    PlaceLabel { x: u16, y: u16, text: [u8; 8] },
}
