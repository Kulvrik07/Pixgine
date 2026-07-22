use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

/// Manages audio playback
pub struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sinks: HashMap<String, Sink>,
    music_sink: Option<Sink>,
    master_volume: f32,
}

impl AudioManager {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;

        Ok(Self {
            _stream,
            stream_handle,
            sinks: HashMap::new(),
            music_sink: None,
            master_volume: 1.0,
        })
    }

    /// Play a sound effect from a file path
    pub fn play_sfx(&mut self, path: &str) -> Result<()> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let sink = Sink::try_new(&self.stream_handle)?;
        sink.set_volume(self.master_volume);
        sink.append(source);
        self.sinks.insert(path.to_string(), sink);
        Ok(())
    }

    /// Play background music (looping)
    pub fn play_music(&mut self, path: &str) -> Result<()> {
        // Stop current music
        if let Some(sink) = &self.music_sink {
            sink.stop();
        }

        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?.repeat_infinite();
        let sink = Sink::try_new(&self.stream_handle)?;
        sink.set_volume(self.master_volume * 0.5); // Music slightly quieter
        sink.append(source);
        self.music_sink = Some(sink);
        Ok(())
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    pub fn stop_all(&mut self) {
        if let Some(sink) = &self.music_sink {
            sink.stop();
        }
        self.sinks.clear();
    }
}