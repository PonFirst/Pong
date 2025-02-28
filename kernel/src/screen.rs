// Original code from rust-osdev/bootloader crate https://github.com/rust-osdev/bootloader

use core::{fmt, ptr};
use noto_sans_mono_bitmap::{FontWeight, get_raster, RasterizedChar};
use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use noto_sans_mono_bitmap::RasterHeight::Size16;
use kernel::RacyCell;

static WRITER: RacyCell<Option<ScreenWriter>> = RacyCell::new(None);
pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let writer = unsafe { WRITER.get_mut() }.as_mut().unwrap();
        writer.write_str(s)
    }
}

pub fn screenwriter() -> &'static mut ScreenWriter {
    let writer = unsafe { WRITER.get_mut() }.as_mut().unwrap();
    writer
}


pub fn init(buffer: &'static mut FrameBuffer) {
    let info = buffer.info();
    let framebuffer = buffer.buffer_mut();
    let writer = ScreenWriter::new(framebuffer, info);
    *unsafe { WRITER.get_mut() } = Some(writer);
}

/// Additional vertical space between lines
const LINE_SPACING: usize = 0;

pub struct ScreenWriter {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
}

impl ScreenWriter {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
        };
        logger.clear();
        logger
    }

    fn newline(&mut self) {
        self.y_pos += Size16 as usize + LINE_SPACING;
        self.carriage_return()
    }

    fn carriage_return(&mut self) {
        self.x_pos = 0;
    }

    /// Erases all text on the screen.
    pub fn clear(&mut self) {
        self.x_pos = 0;
        self.y_pos = 0;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width.into()
    }

    fn height(&self) -> usize {
        self.info.height.into()
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                match get_raster(c, FontWeight::Regular, Size16) {
                    Some(bitmap_char) => {
                        if self.x_pos + bitmap_char.width() > self.width() {
                            self.newline();
                        }
                        if self.y_pos + bitmap_char.height() > self.height() {
                            self.clear();
                        }
                        self.write_rendered_char(bitmap_char);
                    },
                    None => {}
                }
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width();
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * usize::from(self.info.stride) + x;
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [intensity / 4, intensity, intensity / 2, 0],
            PixelFormat::Bgr => [intensity / 2, intensity, intensity / 4, 0],
            other => {
                // set a supported (but invalid) pixel format before panicking to avoid a double
                // panic; it might not be readable though
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * usize::from(bytes_per_pixel);
        self.framebuffer[byte_offset..(byte_offset + usize::from(bytes_per_pixel))]
            .copy_from_slice(&color[..usize::from(bytes_per_pixel)]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        let pixel_offset = y * usize::from(self.info.stride) + x;
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [r, g, b, 0],
            PixelFormat::Bgr => [b, g, r, 0],
            other => {
                // set a supported (but invalid) pixel format before panicking to avoid a double
                // panic; it might not be readable though
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * usize::from(bytes_per_pixel);
        self.framebuffer[byte_offset..(byte_offset + usize::from(bytes_per_pixel))]
            .copy_from_slice(&color[..usize::from(bytes_per_pixel)]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

}

unsafe impl Send for ScreenWriter {}
unsafe impl Sync for ScreenWriter {}

impl fmt::Write for ScreenWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}