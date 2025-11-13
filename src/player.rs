use anyhow::Result;
use evdev::KeyCode;
use rodio::{OutputStream, OutputStreamBuilder, Sink, play, source::EmptyCallback};
use std::{fs::File, io::BufReader, path::Path};

pub struct Player {
    stream_handle: OutputStream,
    sink: Option<Sink>,
}

impl Player {
    pub fn new() -> Result<Self> {
        let stream_handle = OutputStreamBuilder::open_default_stream()?;
        Ok(Player {
            stream_handle,
            sink: None,
        })
    }

    pub fn play<F>(&mut self, audio: &Path, end_cb: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(KeyCode) + Send + 'static,
    {
        let mixer = self.stream_handle.mixer();
        let file = File::open(audio)?;
        let sink = play(mixer, BufReader::new(file))?;

        sink.append(EmptyCallback::new(Box::new(move || {
            println!("End of stream");
            end_cb(KeyCode::BTN_START);
        })));

        sink.set_volume(0.2);
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
}
