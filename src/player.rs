use anyhow::Result;
use rodio::{OutputStream, OutputStreamBuilder, Sink, play, source::EmptyCallback};
use std::{fs::File, io::BufReader, path::Path};

pub struct Player {
    stream_handle: OutputStream,
    sink: Option<Sink>,
    volume: f32,
}

impl Player {
    pub fn new() -> Result<Self> {
        let stream_handle = OutputStreamBuilder::open_default_stream()?;
        Ok(Player {
            stream_handle,
            sink: None,
            volume: 0.2,
        })
    }

    pub fn play<F>(&mut self, audio: &Path, end_cb: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() + Send + 'static,
    {
        let mixer = self.stream_handle.mixer();
        let file = File::open(audio)?;
        let sink = play(mixer, BufReader::new(file))?;

        sink.append(EmptyCallback::new(Box::new(move || {
            println!("End of stream");
            end_cb();
        })));

        sink.set_volume(self.volume);
        self.sink = Some(sink);

        Ok(())
    }

    pub fn toggle_pause(&self) {
        if let Some(sink) = &self.sink {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        match &self.sink {
            Some(sink) => sink.is_paused(),
            None => false,
        }
    }

    pub fn volume_up(&mut self) {
        if let Some(sink) = &self.sink {
            let mut volume = sink.volume();
            if volume < 1.0 {
                volume = volume + 0.1;
            }
            self.volume = volume;
            sink.set_volume(volume);
        }
    }

    pub fn volume_down(&mut self) {
        if let Some(sink) = &self.sink {
            let mut volume = sink.volume();
            if volume > 0.2 {
                volume = volume - 0.1;
            }
            self.volume = volume;
            sink.set_volume(volume);
        }
    }
}
