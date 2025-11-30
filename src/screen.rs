/* Contelia
 * Copyright (C) 2025  Mathieu Schroeter <mathieu@schroetersa.ch>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use anyhow::Result;
use framebuffer::Framebuffer;
use image::GenericImageView;
use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::Path,
};

pub struct Screen {
    dev: String,
    fb: Framebuffer,
}

impl Screen {
    pub fn new(fb: &Path) -> Result<Self> {
        let dev = fb
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let fb = Framebuffer::new(fb)?;
        Ok(Screen { dev, fb })
    }

    pub fn off(&self) -> io::Result<()> {
        fs::write(format!("/sys/class/graphics/{}/blank", self.dev), "1")
    }

    pub fn on(&self) -> io::Result<()> {
        fs::write(format!("/sys/class/graphics/{}/blank", self.dev), "0")
    }

    pub fn draw(
        &mut self,
        image: &File,
        format: image::ImageFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.fb.var_screen_info.xres;
        let height = self.fb.var_screen_info.yres;
        let line_length = self.fb.fix_screen_info.line_length;
        let reader = BufReader::new(image);
        let img = image::load(reader, format)?.resize(
            width,
            height,
            image::imageops::FilterType::CatmullRom,
        );

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
