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
    previous_paddle_left_pos: usize,   // Track previous position of the left paddle
    previous_paddle_right_pos: usize,  // Track previous position of the right paddle
}

impl ScreenWriter {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
            previous_paddle_left_pos: 0,  // Initializing previous paddle positions
            previous_paddle_right_pos: 0, // Initializing previous paddle positions
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

    pub fn width(&self) -> usize {
        self.info.width.into()
    }

    pub fn height(&self) -> usize {
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

    pub fn draw_zero(&mut self, x: usize, y: usize, size: usize) {
        let thickness = size / 5;
    
        // Draw the top horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + t, 255, 255, 255);
            }
        }
    
        // Draw the bottom horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + size - thickness + t, 255, 255, 255);
            }
        }
    
        // Draw the left vertical line
        for dy in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + t, y + dy, 255, 255, 255);
            }
        }
    
        // Draw the right vertical line
        for dy in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + size - thickness + t, y + dy, 255, 255, 255);
            }
        }
    }
    
    pub fn draw_one(&mut self, x: usize, y: usize, size: usize) {
        let thickness = size / 5;
    
        for dy in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + size / 2 + t, y + dy, 255, 255, 255);
            }
        }
    }
    
    pub fn draw_two(&mut self, x: usize, y: usize, size: usize) {
        let thickness = size / 5;
    
        // Top horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + t, 255, 255, 255);
            }
        }

        // Right vertical line
        let vertical_height = size / 2;
        for dy in 0..vertical_height {
            for dx in 0..thickness {
                screenwriter().draw_pixel(x + size - thickness + dx, y + dy, 255, 255, 255);
            }
        }
    
        // Middle horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + size / 2 - thickness / 2 + t, 255, 255, 255);
            }
        }
    
        // Bottom horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + size - thickness + t, 255, 255, 255);
            }
        }
    
        // Draw the bottom-left vertical line
        for dy in 0..size / 2 {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + t, y + size / 2 + dy, 255, 255, 255);
            }
        }
    }

    pub fn draw_three(&mut self, x: usize, y: usize, size: usize) {
        let thickness = size / 5;
    
        // Top horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + t, 255, 255, 255);
            }
        }
    
        // Middle horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + size / 2 - thickness / 2 + t, 255, 255, 255);
            }
        }
    
        // Bottom horizontal line
        for dx in 0..size {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + dx, y + size - thickness + t, 255, 255, 255);
            }
        }
    
        // Right vertical line (upper half)
        for dy in 0..size / 2 {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + size - thickness + t, y + dy, 255, 255, 255);
            }
        }
    
        // Right vertical line (lower half)
        for dy in 0..size / 2 {
            for t in 0..thickness {
                screenwriter().draw_pixel(x + size - thickness + t, y + size / 2 + dy, 255, 255, 255);
            }
        }
    }

    pub fn clear_score(&mut self, x: usize, y: usize, size: usize) {
        let width = size;  // Total pixel width of the digit
        let height = size; // Total pixel height of the digit
        for dx in 0..width {
            for dy in 0..height {
                self.draw_pixel(x + dx, y + dy, 0, 0, 0); // Clear with black
            }
        }
    }
    
    

    pub fn set_position(&mut self, x: usize, y: usize) {
        self.x_pos = x;
        self.y_pos = y;
    }

    pub fn draw_pong_pad(&mut self, x_pos: usize, y_pos: usize, height: usize, width: usize) {
        for y in y_pos..(y_pos + height) {
            for x in x_pos..(x_pos + width) {
                self.draw_pixel(x, y, 255, 255, 255); // White color for the pad
            }
        }
    }

    pub fn draw_pong_game(&mut self) {
        // Define the size of the pads
        let paddle_width = 10;
        let paddle_height = 60;

        let paddle_left_pos = unsafe { crate::PADDLE_LEFT };
        let paddle_right_pos = unsafe { crate::PADDLE_RIGHT };

        let paddle_left_x = 10;
        let paddle_right_x = self.width() - paddle_width - 10;

        unsafe {
            // Only clear and redraw if the positions have changed
            if paddle_left_pos != self.previous_paddle_left_pos {
                self.clear_pong_pad(paddle_left_x, self.previous_paddle_left_pos, paddle_height, paddle_width);
                self.draw_pong_pad(paddle_left_x, paddle_left_pos, paddle_height, paddle_width);
                self.previous_paddle_left_pos = paddle_left_pos;
            }
            
            if paddle_right_pos != self.previous_paddle_right_pos {
                self.clear_pong_pad(paddle_right_x, self.previous_paddle_right_pos, paddle_height, paddle_width);
                self.draw_pong_pad(paddle_right_x, paddle_right_pos, paddle_height, paddle_width);
                self.previous_paddle_right_pos = paddle_right_pos;
            }
            
        }
    }

    pub fn draw_ball(&mut self, x: usize, y: usize, size: usize) {
        for dx in 0..size {
            for dy in 0..size {
                let px = x + dx;
                let py = y + dy;
                if px < self.width() && py < self.height() {
                    self.draw_pixel(px, py, 0xff, 0xff, 0x00);
                }
            }
        }
    }

    pub fn draw_mid_line(&mut self) {
        let mid_line_width = 5;
        let mid_line_height = 10;
        let total_lines = 20;
        for i in 0..total_lines {
            let mid_line_x = (self.width() - mid_line_width) / 2;
            let mid_line_y = i * (self.height() / total_lines);
            for y in mid_line_y..(mid_line_y + mid_line_height) {
                for x in mid_line_x..(mid_line_x + mid_line_width) {
                    self.draw_pixel(x, y, 255, 255, 255); // White color for the mid line
                }
            }
        }
    }


    pub fn clear_pong_pad(&mut self, x_pos: usize, y_pos: usize, height: usize, width: usize) {
        for y in y_pos..(y_pos + height) {
            for x in x_pos..(x_pos + width) {
                self.draw_pixel(x, y, 0, 0, 0); // Clear with black
            }
        }
    }

    pub fn clear_ball(&mut self, ball_x: usize, ball_y: usize, ball_size: usize) {
        for y in ball_y..(ball_y + ball_size) {
            for x in ball_x..(ball_x + ball_size) {
                self.draw_pixel(x, y, 0, 0, 0); // Clear with black
            }
        }
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
