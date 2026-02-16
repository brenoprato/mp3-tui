mod app;
mod ui;
mod player;

use player::MusicPlayer;
use app::App;
use std::{result, time::Duration};
use color_eyre::eyre::{Ok,Result};
use ratatui::{init, restore,DefaultTerminal, Terminal, crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind}};


fn main() -> Result<()> { 
    color_eyre::install()?;

    //player init
    let mut music_player: MusicPlayer = MusicPlayer::new().unwrap();

    //App & tui init
    let mut app: App = App::new();
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &mut music_player);
    ratatui::restore();
    result 
}

//main loop
pub fn run(terminal: &mut DefaultTerminal, app: &mut App, music_player: &mut MusicPlayer)-> Result<()>{

    loop {
        MusicPlayer::update_state(music_player);
        if app.current_song_path != music_player.current_song_path{
            app.current_song_path = None;
        }
        terminal.draw(|frame| ui::render(frame, app, music_player))?;
        
        //keybinds
        if event::poll(Duration::from_millis(16))?{
            if let Event::Key(key) = event::read()?{
                match key.code {
                    event::KeyCode::Esc => {
                        break;
                    }
                    event::KeyCode::Down => {
                        if app.select_index < app.archieves.len() - 1{
                            app.select_index = app.select_index + 1;
                        }
                    }
                    event::KeyCode::Up => {
                        if app.select_index > 0{
                            app.select_index = app.select_index - 1;
                        }
                    }
                    event::KeyCode::Enter => {
                        if let Some(selected_entry) = app.archieves.get(app.select_index) {
                            if selected_entry.is_dir {
                                app.current_path = selected_entry.path.clone();
                                app.reload();
                                app.select_index = 0;
                            } else if App::is_audio_file(&selected_entry.path) {
                                let selected_path = selected_entry.path.clone();
                                let selected_name = selected_entry.name.clone();

                                if let Some(current_path) = &app.current_song_path {
                                    if current_path == &selected_path {
                                        if music_player.is_paused() {
                                            music_player.resume();
                                        } else {
                                            music_player.pause();
                                        }
                                    } else {
                                        if let Err(e) = music_player.play_file(selected_path.clone()) {

                                        } else {
                                            app.current_song_path = Some(selected_path);
                                        }
                                    }
                                } else {
                                    if let Err(e) = music_player.play_file(selected_path.clone()) {

                                    } else {
                                        app.current_song_path = Some(selected_path);
                                    }
                                }
                            }
                        }
                    }
                    event::KeyCode::Char('1') => {
                        app.ui_mode = app::UImode::Default;
                    }
                    event::KeyCode::Char('2') => {
                        app.ui_mode = app::UImode::Full_screen_player;
                    }
                    _ => {}
                }
            }
        }    
    }

    Ok(())
}