use anyhow::Result;
use rodio::{OutputStream, OutputStreamBuilder, Sink, play};
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

    pub fn play(&mut self, audio: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mixer = self.stream_handle.mixer();
        let file = File::open(audio)?;
        let sink = play(mixer, BufReader::new(file))?;

        sink.set_volume(0.8);
        self.sink = Some(sink);

        Ok(())
    }
}
