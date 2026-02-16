use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use rodio::{Decoder, OutputStream, Sink, Source};

use crate::app;
use app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongState {
    Repeat_song,
    Shuffle,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

pub struct MusicPlayer {
    pub current_song_path: Option<PathBuf>,
    pub current_song: Option<String>,
    pub queue: Vec<PathBuf>,
    pub song_state: SongState,
    pub state: PlaybackState,
    pub start_time: Option<Instant>,
    pub paused_offset: Option<Duration>,
    pub current_duration: Option<Duration>,
    _stream_handle: OutputStream,
    sink: Sink,
    // Timing fields
}

impl MusicPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());

        Ok(Self {
            current_song_path: None,
            current_song: None,
            queue: Vec::new(),
            state: PlaybackState::Stopped,
            song_state: SongState::Default,
            start_time: None,
            paused_offset: None,
            current_duration: None,
            _stream_handle: stream_handle,
            sink: sink,
        })
    }

    pub fn update_state(&mut self) {
        if self.state == PlaybackState::Playing && self.sink.empty() {
            self.state = PlaybackState::Stopped;
            self.current_song_path = None;
            self.current_song = None;
            self.current_duration = None;
            self.start_time = None;
            self.paused_offset = None;
        }
    }

    pub fn play_file(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(&path)?;
    let source = Decoder::new(BufReader::new(file))?;
    // Store total duration before appending to sink
    self.current_duration = source.total_duration();
    self.sink.stop();
    self.sink.append(source);
    self.current_song_path = Some(path.clone());
    self.current_song = Some(
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
    self.state = PlaybackState::Playing;
    self.start_time = Some(Instant::now());
    self.paused_offset = None;

    Ok(())
}

    pub fn enqueue(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(&path)?;
        let source = Decoder::new(BufReader::new(file))?;
        self.sink.append(source);
        self.queue.push(path);
        Ok(())
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            if let Some(start) = self.start_time {
                self.paused_offset = Some(start.elapsed());
            }
            self.start_time = None;
            self.sink.pause();
            self.state = PlaybackState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == PlaybackState::Paused {
            if let Some(offset) = self.paused_offset.take() {
                self.start_time = Some(Instant::now() - offset);
            }
            self.sink.play();
            self.state = PlaybackState::Playing;
        }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_song_path = None;
        self.current_song = None;
        self.queue.clear();
        self.state = PlaybackState::Stopped;
        self.current_duration = None;
        self.start_time = None;
        self.paused_offset = None;
    }

    pub fn is_paused(&self) -> bool {
        self.state == PlaybackState::Paused
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    pub fn skip(&mut self) {
        self.sink.stop();
        // Optionally handle next in queue, but for now just stop
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&self, vol: f32) {
        self.sink.set_volume(vol);
    }

    /// Returns the current playback position if a song is playing or paused.
    pub fn current_position(&self) -> Option<Duration> {
        match self.state {
            PlaybackState::Playing => self.start_time.map(|t| t.elapsed()),
            PlaybackState::Paused => self.paused_offset,
            PlaybackState::Stopped => None,
        }
    }

    /// Returns the total duration of the current song, if known.
    pub fn current_duration(&self) -> Option<Duration> {
        self.current_duration
    }
}