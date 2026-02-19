use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use rodio::{Decoder, OutputStream, Sink, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

pub struct MusicPlayer {
    pub current_song_path: Option<PathBuf>,
    pub current_song_name: Option<String>,
    pub state: PlaybackState,
    current_duration: Option<Duration>,
    duration_rx: Option<Receiver<DurationUpdate>>,
    _stream: OutputStream,
    sink: Sink,
}

struct DurationUpdate {
    path: PathBuf,
    duration: Option<Duration>,
}

impl MusicPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(&stream.mixer());

        Ok(Self {
            current_song_path: None,
            current_song_name: None,
            state: PlaybackState::Stopped,
            current_duration: None,
            duration_rx: None,
            _stream: stream,
            sink,
        })
    }

    pub fn update_state(&mut self) {
        if let Some(rx) = &self.duration_rx {
            if let Ok(update) = rx.try_recv() {
                self.duration_rx = None;
                if self.current_song_path.as_ref() == Some(&update.path) {
                    self.current_duration = update.duration;
                }
            }
        }

        if self.state != PlaybackState::Stopped && self.sink.empty() {
            self.clear_track_state();
        }
    }

    pub fn play_file(
        &mut self,
        path: PathBuf,
        prefetched_duration: Option<Duration>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(&path)?;
        let source = Decoder::new(BufReader::new(file))?;

        self.current_duration = source.total_duration().or(prefetched_duration);
        self.duration_rx = None;
        self.sink.stop();
        self.sink.append(source);
        self.sink.play();

        self.current_song_path = Some(path.clone());
        self.current_song_name = Some(
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        self.state = PlaybackState::Playing;

        if self.current_duration.is_none() {
            let (tx, rx) = mpsc::channel();
            let duration_path = path.clone();
            thread::spawn(move || {
                let duration = probe_duration(&duration_path);
                let _ = tx.send(DurationUpdate {
                    path: duration_path,
                    duration,
                });
            });
            self.duration_rx = Some(rx);
        }

        Ok(())
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.sink.pause();
            self.state = PlaybackState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == PlaybackState::Paused {
            self.sink.play();
            self.state = PlaybackState::Playing;
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused => self.resume(),
            PlaybackState::Stopped => {}
        }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.clear_track_state();
    }

    pub fn is_playing_track(&self, path: &Path) -> bool {
        self.current_song_path.as_ref().map(PathBuf::as_path) == Some(path)
    }

    pub fn current_position(&self) -> Option<Duration> {
        if self.state == PlaybackState::Stopped {
            None
        } else {
            Some(self.sink.get_pos())
        }
    }

    pub fn current_duration(&self) -> Option<Duration> {
        self.current_duration
    }

    fn clear_track_state(&mut self) {
        self.current_song_path = None;
        self.current_song_name = None;
        self.current_duration = None;
        self.duration_rx = None;
        self.state = PlaybackState::Stopped;
    }
}

pub fn probe_duration(path: &Path) -> Option<Duration> {
    let file = File::open(path).ok()?;
    let decoder = Decoder::new(BufReader::new(file)).ok()?;
    let sample_rate = decoder.sample_rate() as f64;
    let channels = decoder.channels() as f64;
    if sample_rate <= 0.0 || channels <= 0.0 {
        return None;
    }

    let total_samples = decoder.count() as f64;
    let total_seconds = total_samples / (sample_rate * channels);
    Some(Duration::from_secs_f64(total_seconds.max(0.0)))
}
