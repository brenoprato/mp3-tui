use crate::player::probe_duration;
use std::collections::{HashMap, HashSet};
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Default,
    FullScreenPlayer,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug)]
pub struct App {
    pub ui_mode: UiMode,
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub status: Option<String>,
    duration_cache: HashMap<PathBuf, Option<Duration>>,
    duration_rx: Option<Receiver<DurationUpdate>>,
    duration_db: Option<sled::Db>,
}

#[derive(Debug)]
struct DurationUpdate {
    path: PathBuf,
    duration: Option<Duration>,
}

impl App {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let duration_db = sled::open(".mp3-tui-cache").ok();
        let mut app = Self {
            ui_mode: UiMode::Default,
            current_path: current_dir,
            entries: Vec::new(),
            selected_index: 0,
            status: None,
            duration_cache: HashMap::new(),
            duration_rx: None,
            duration_db,
        };
        app.reload();
        app
    }

    pub fn reload(&mut self) {
        self.entries.clear();

        if let Some(parent) = self.current_path.parent() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }

        if let Ok(read_dir) = fs::read_dir(&self.current_path) {
            for entry in read_dir.filter_map(Result::ok) {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();
                    let is_visible_dir = metadata.is_dir() && !Self::is_hidden(&entry);
                    let is_audio = Self::is_audio_file(&path);
                    if is_visible_dir || is_audio {
                        self.entries.push(FileEntry {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path,
                            is_dir: metadata.is_dir(),
                        });
                    }
                }
            }
        }

        self.entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        if self.entries.is_empty() {
            self.selected_index = 0;
            self.duration_rx = None;
            self.sync_folder_db(&HashSet::new());
            return;
        }

        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len() - 1;
        }

        let folder_audio_paths = self.current_folder_audio_paths();
        self.sync_folder_db(&folder_audio_paths);
        self.load_cached_folder_durations(&folder_audio_paths);
        self.start_duration_prefetch(folder_audio_paths);
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.entries.len() - 1);
        }
    }

    pub fn enter_directory(&mut self, path: PathBuf) {
        self.current_path = path;
        self.selected_index = 0;
        self.reload();
    }

    pub fn cached_duration(&self, path: &Path) -> Option<Duration> {
        self.duration_cache.get(path).and_then(|value| *value)
    }

    pub fn update_background_jobs(&mut self) {
        let mut disconnect = false;

        if let Some(rx) = &self.duration_rx {
            loop {
                match rx.try_recv() {
                    Ok(update) => {
                        self.duration_cache
                            .insert(update.path.clone(), update.duration);
                        self.write_duration_to_db(&update.path, update.duration);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnect = true;
                        break;
                    }
                }
            }
        }

        if disconnect {
            self.duration_rx = None;
        }
    }

    pub fn is_audio_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac"
                )
            })
            .unwrap_or(false)
    }

    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|name| name.starts_with('.'))
            .unwrap_or(false)
    }

    fn start_duration_prefetch(&mut self, folder_audio_paths: HashSet<PathBuf>) {
        let paths_to_scan: Vec<PathBuf> = folder_audio_paths
            .into_iter()
            .filter(|path| !self.duration_cache.contains_key(path))
            .collect();

        if paths_to_scan.is_empty() {
            self.duration_rx = None;
            return;
        }

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for path in paths_to_scan {
                let duration = probe_duration(&path);
                if tx.send(DurationUpdate { path, duration }).is_err() {
                    break;
                }
            }
        });
        self.duration_rx = Some(rx);
    }

    fn current_folder_audio_paths(&self) -> HashSet<PathBuf> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path.clone())
            .collect()
    }

    fn sync_folder_db(&mut self, folder_audio_paths: &HashSet<PathBuf>) {
        let Some(db) = &self.duration_db else {
            return;
        };

        let mut stale_keys = Vec::new();
        for item in db.iter().flatten() {
            let (key, _) = item;
            let key_path = PathBuf::from(String::from_utf8_lossy(&key).to_string());
            if key_path.parent() == Some(self.current_path.as_path())
                && !folder_audio_paths.contains(&key_path)
            {
                stale_keys.push(key);
            }
        }

        for key in stale_keys {
            let _ = db.remove(key);
        }
        let _ = db.flush();
    }

    fn load_cached_folder_durations(&mut self, folder_audio_paths: &HashSet<PathBuf>) {
        let Some(db) = &self.duration_db else {
            return;
        };

        for path in folder_audio_paths {
            let key = path.to_string_lossy().to_string();
            if let Ok(Some(raw)) = db.get(key.as_bytes()) {
                self.duration_cache
                    .insert(path.clone(), decode_duration(&raw));
            }
        }
    }

    fn write_duration_to_db(&self, path: &Path, duration: Option<Duration>) {
        let Some(db) = &self.duration_db else {
            return;
        };
        let key = path.to_string_lossy().to_string();
        let _ = db.insert(key.as_bytes(), encode_duration(duration));
    }
}

fn encode_duration(duration: Option<Duration>) -> Vec<u8> {
    match duration {
        Some(value) => {
            let millis = value.as_millis().min(u128::from(u64::MAX)) as u64;
            let mut bytes = Vec::with_capacity(9);
            bytes.push(1);
            bytes.extend_from_slice(&millis.to_le_bytes());
            bytes
        }
        None => vec![0],
    }
}

fn decode_duration(raw: &[u8]) -> Option<Duration> {
    if raw.first().copied() != Some(1) || raw.len() != 9 {
        return None;
    }
    let mut millis_bytes = [0_u8; 8];
    millis_bytes.copy_from_slice(&raw[1..9]);
    Some(Duration::from_millis(u64::from_le_bytes(millis_bytes)))
}
