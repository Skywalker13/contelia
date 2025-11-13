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
        let img =
            image::open(image)?.resize(width, height, image::imageops::FilterType::CatmullRom);

        let (_w, h) = img.dimensions();
        let h_offset = if height > h { (height - h) / 2 } else { 0 };
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * line_length / 2) + x) as usize;

                if y >= h_offset && y - h_offset < h {
                    let pixel = img.get_pixel(x, y - h_offset);
                    let rgb565 = ((pixel[0] as u16 >> 3) << 11)
                        | ((pixel[1] as u16 >> 2) << 5)
                        | (pixel[2] as u16 >> 3);

                    self.fb.frame[offset * 2] = (rgb565 & 0xFF) as u8;
                    self.fb.frame[offset * 2 + 1] = (rgb565 >> 8) as u8;
                } else {
                    self.fb.frame[offset * 2] = 0 as u8;
                    self.fb.frame[offset * 2 + 1] = 0 as u8;
                };
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.fb.frame.fill(0);
        Ok(())
    }
}
