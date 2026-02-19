mod app;
mod player;
mod ui;

use app::App;
use app::UiMode;
use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyCode};
use player::MusicPlayer;
use ratatui::DefaultTerminal;
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut music_player = MusicPlayer::new().map_err(|err| eyre!(err.to_string()))?;
    let mut app = App::new();
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &mut music_player);
    ratatui::restore();
    result
}

pub fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    music_player: &mut MusicPlayer,
) -> Result<()> {
    loop {
        app.update_background_jobs();
        music_player.update_state();
        terminal.draw(|frame| ui::render(frame, app, music_player))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Down => {
                        app.move_down();
                    }
                    KeyCode::Up => {
                        app.move_up();
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = app.selected_entry().cloned() {
                            if selected.is_dir {
                                app.enter_directory(selected.path);
                                app.status = None;
                            } else if music_player.is_playing_track(&selected.path) {
                                music_player.toggle_pause();
                                app.status = None;
                            } else {
                                let prefetched_duration = app.cached_duration(&selected.path);
                                match music_player
                                    .play_file(selected.path.clone(), prefetched_duration)
                                {
                                    Ok(()) => app.status = None,
                                    Err(err) => {
                                        app.status = Some(format!("playback error: {err}"));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('1') => {
                        app.ui_mode = UiMode::Default;
                    }
                    KeyCode::Char('2') => {
                        app.ui_mode = UiMode::FullScreenPlayer;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
