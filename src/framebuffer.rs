//! PSRAM-backed framebuffer with per-pixel change detection.
//!
//! All page drawing targets this RAM buffer instead of the SPI display.
//! After drawing completes, only the rectangular region containing changed
//! pixels is flushed to the hardware display in a single SPI transaction.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::convert::Infallible;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use log::debug;

use crate::ui::{DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX};

/// Total number of pixels in the framebuffer (320 x 240 = 76,800).
const PIXEL_COUNT: usize = DISPLAY_WIDTH_PX as usize * DISPLAY_HEIGHT_PX as usize;

/// Bounding box of pixels that have changed since the last flush.
#[derive(Debug, Clone, Copy)]
struct DirtyRect {
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
}

impl DirtyRect {
    /// Expand the dirty region to include the given pixel coordinate.
    fn expand(&mut self, x: usize, y: usize) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    /// Create a new dirty rect covering a single pixel.
    fn from_point(x: usize, y: usize) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }
}

/// PSRAM-backed framebuffer implementing `DrawTarget<Color = Rgb565>`.
///
/// Heap-allocates a 320x240x2 = 153,600-byte pixel buffer (1.8% of 8MB PSRAM).
/// Tracks a dirty bounding box so that only changed pixels are flushed to the
/// hardware display.
pub struct FrameBuffer {
    pixels: Vec<Rgb565>,
    dirty: Option<DirtyRect>,
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuffer {
    /// Allocate a new framebuffer filled with black pixels.
    ///
    /// The allocation lands in PSRAM via the global allocator.
    pub fn new() -> Self {
        Self {
            pixels: vec![Rgb565::BLACK; PIXEL_COUNT],
            dirty: None,
        }
    }

    /// Write a single pixel, expanding the dirty rect only if the color changed.
    #[inline]
    fn set_pixel(&mut self, x: usize, y: usize, color: Rgb565) {
        let idx = y * DISPLAY_WIDTH_PX as usize + x;
        if self.pixels[idx] != color {
            self.pixels[idx] = color;
            match &mut self.dirty {
                Some(rect) => rect.expand(x, y),
                None => self.dirty = Some(DirtyRect::from_point(x, y)),
            }
        }
    }

    /// Flush the dirty region to a hardware display, then reset the dirty state.
    ///
    /// Only the bounding rectangle of changed pixels is sent over SPI via
    /// `fill_contiguous`. If nothing changed, this is a no-op.
    pub fn flush<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let Some(rect) = self.dirty.take() else {
            return Ok(());
        };

        let width = rect.max_x - rect.min_x + 1;
        let height = rect.max_y - rect.min_y + 1;

        debug!(
            "Flushing {}x{} dirty region at ({}, {})",
            width, height, rect.min_x, rect.min_y
        );

        let area = Rectangle::new(
            Point::new(rect.min_x as i32, rect.min_y as i32),
            Size::new(width as u32, height as u32),
        );

        // Borrow the pixel slice so the closure captures a shared reference,
        // avoiding the `FnMut` escaping-reference issue with `&mut self`.
        let pixels = &self.pixels;
        let stride = DISPLAY_WIDTH_PX as usize;
        let pixel_iter = (rect.min_y..=rect.max_y).flat_map(move |y| {
            let row_start = y * stride + rect.min_x;
            pixels[row_start..row_start + width].iter().copied()
        });

        display.fill_contiguous(&area, pixel_iter)
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(DISPLAY_WIDTH_PX as u32, DISPLAY_HEIGHT_PX as u32)
    }
}

impl DrawTarget for FrameBuffer {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let w = DISPLAY_WIDTH_PX as usize;
        let h = DISPLAY_HEIGHT_PX as usize;

        for Pixel(coord, color) in pixels {
            let x = coord.x;
            let y = coord.y;
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                self.set_pixel(x as usize, y as usize, color);
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let w = DISPLAY_WIDTH_PX as usize;
        let h = DISPLAY_HEIGHT_PX as usize;

        // Clamp the area to display bounds
        let area_x = area.top_left.x.max(0) as usize;
        let area_y = area.top_left.y.max(0) as usize;
        let area_w = area.size.width as usize;
        let area_h = area.size.height as usize;

        let mut colors = colors.into_iter();
        for row in 0..area_h {
            let y = area_y + row;
            for col in 0..area_w {
                let x = area_x + col;
                if let Some(color) = colors.next()
                    && x < w
                    && y < h
                {
                    self.set_pixel(x, y, color);
                }
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let w = DISPLAY_WIDTH_PX as usize;
        let h = DISPLAY_HEIGHT_PX as usize;

        let x_start = (area.top_left.x.max(0) as usize).min(w);
        let y_start = (area.top_left.y.max(0) as usize).min(h);
        let x_end = ((area.top_left.x as usize).saturating_add(area.size.width as usize)).min(w);
        let y_end = ((area.top_left.y as usize).saturating_add(area.size.height as usize)).min(h);

        for y in y_start..y_end {
            for x in x_start..x_end {
                self.set_pixel(x, y, color);
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let w = DISPLAY_WIDTH_PX as usize;
        let h = DISPLAY_HEIGHT_PX as usize;

        for y in 0..h {
            for x in 0..w {
                self.set_pixel(x, y, color);
            }
        }
        Ok(())
    }
}
