use std::collections::btree_map::Entry;
use std::default;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::fs::{self, DirEntry, File};

pub enum UImode{
    Default,
    Full_screen_player,
}
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}
pub struct App{
    pub ui_mode: UImode,
    pub exit: bool,
    pub current_path: PathBuf,
    pub archieves: Vec<FileEntry>,
    pub select_index: usize,
    pub current_song_path: Option<PathBuf>,
}

impl App{
    pub fn new() -> Self{
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut app = Self {
            ui_mode: UImode::Default,
            exit: false,
            current_path: current_dir,
            archieves: Vec::new(),
            select_index: 0,
            current_song_path: None,
        };

        app.reload();

        app
    }

    //scan folder
    pub fn reload(&mut self) {
        self.archieves.clear();

        //scan current path
        if self.current_path.parent().is_some(){
            self.archieves.push(FileEntry {
                name: "..".to_string(),
                path: self.current_path.parent().unwrap().to_path_buf(),
                is_dir: true,
            });
        }

        //scan archieves in the path
        if let Ok(read_dir) = fs::read_dir(self.current_path.clone()){
            for entry in read_dir.filter_map(Result::ok){
                if let Ok(metadata) = entry.metadata(){
                    if metadata.is_dir() && !Self::is_hidden(&entry) || Self::is_audio_file(&entry.path()) {
                        self.archieves.push(FileEntry { 
                            name: entry.file_name().to_string_lossy().to_string(), 
                            path: entry.path(), 
                            is_dir: metadata.is_dir(), 
                        });
                    }
                }
            }
        }

        //ordering rule
        self.archieves.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        if self.select_index >= self.archieves.len() && !self.archieves.is_empty() {
            self.select_index = self.archieves.len() - 1;
        }
    }

    pub fn is_audio_file(path: &Path) -> bool {
        if let Some(ext) = path.extension(){
            let audio_exts = ["mp3", "wav", "flac", "ogg", "m4a", "aac"];
            return ext.to_str()
            .map(|e| audio_exts.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false);
        }   
        false
    }

    fn is_hidden(entry: &DirEntry<>) -> bool {
        entry.file_name()
            .to_str()
            .map(|name| name.starts_with('.'))
            .unwrap_or(false)
    }
}