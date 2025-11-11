use anyhow::Result;
use framebuffer::Framebuffer;
use image::GenericImageView;
use std::path::Path;

pub struct Renderer {
    fb: Framebuffer,
}

impl Renderer {
    pub fn new(fb: &Path) -> Result<Self> {
        let fb = Framebuffer::new(fb)?;
        Ok(Renderer { fb })
    }

    pub fn blit(&mut self, image: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.fb.var_screen_info.xres;
        let height = self.fb.var_screen_info.yres;
        let line_length = self.fb.fix_screen_info.line_length;
        let img = image::open(image)?.resize_to_fill(
            width,
            height,
            image::imageops::FilterType::CatmullRom,
        );

        let (w, h) = img.dimensions();
        for y in 0..h.min(height) {
            for x in 0..w.min(width) {
                let pixel = img.get_pixel(x, y);
                let offset = ((y * line_length / 2) + x) as usize;

                let rgb565 = ((pixel[0] as u16 >> 3) << 11)
                    | ((pixel[1] as u16 >> 2) << 5)
                    | (pixel[2] as u16 >> 3);

                self.fb.frame[offset * 2] = (rgb565 & 0xFF) as u8;
                self.fb.frame[offset * 2 + 1] = (rgb565 >> 8) as u8;
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.fb.frame.fill(0);
        Ok(())
    }
}
