/* Contelia
 * Copyright (C) 2025-2026  Mathieu Schroeter <mathieu@schroetersa.ch>
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
use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use rodio::{play, source::EmptyCallback};
use std::io::BufReader;

use crate::FileReader;

pub struct Player {
    device_sink: Option<MixerDeviceSink>,
    player: Option<rodio::Player>,
    volume: f32,
}

impl Player {
    pub fn new() -> Result<Self> {
        Ok(Self {
            device_sink: None,
            player: None,
            volume: 0.2,
        })
    }

    pub fn play<F>(
        &mut self,
        audio: FileReader,
        end_cb: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() + Send + 'static,
    {
        self.stop();
        self.device_sink = DeviceSinkBuilder::open_default_sink().ok();

        if let Some(ref device_sink) = self.device_sink {
            let mixer = device_sink.mixer();
            let reader = BufReader::new(audio);
            let sink = play(mixer, reader)?;

            sink.append(EmptyCallback::new(Box::new(move || {
                println!("End of stream");
                end_cb();
            })));

            sink.set_volume(self.volume);
            self.player = Some(sink);
        } else {
            eprintln!("No default stream found");
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.player {
            sink.stop();
        }

        self.player = None;
        self.device_sink = None;
    }

    pub fn toggle_pause(&self) {
        if let Some(sink) = &self.player {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        match &self.player {
            Some(sink) => sink.is_paused(),
            None => false,
        }
    }

    pub fn get_volume(&self) -> usize {
        match &self.player {
            Some(sink) => (sink.volume() * 10.0).round() as usize,
            None => 0,
        }
    }

    pub fn volume_up(&mut self) {
        if let Some(sink) = &self.player {
            let mut volume = sink.volume();
            if volume < 1.0 {
                volume = volume + 0.1;
            }
            self.volume = volume;
            sink.set_volume(volume);
        }
    }

    pub fn volume_down(&mut self) {
        if let Some(sink) = &self.player {
            let mut volume = sink.volume();
            if volume > 0.2 {
                volume = volume - 0.1;
            }
            self.volume = volume;
            sink.set_volume(volume);
        }
    }
}
