//! RGB colour with `0RGB` packing, vendored from the standalone 3D renderer.

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Colour { r, g, b }
    }

    /// Pack into a `0x00RRGGBB` word, the software framebuffer's pixel format.
    pub fn as_0rgb(&self) -> u32 {
        ((self.r as u32) << 16) + ((self.g as u32) << 8) + (self.b as u32)
    }

    pub fn scale(&self, factor: f32) -> Self {
        Colour {
            r: ((self.r as f32) * factor) as u8,
            g: ((self.g as f32) * factor) as u8,
            b: ((self.b as f32) * factor) as u8,
        }
    }
}
