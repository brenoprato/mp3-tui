use crate::app::{App, UiMode};
use crate::player::{MusicPlayer, PlaybackState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};
use std::time::Duration;

pub fn render(frame: &mut Frame, app: &App, player: &MusicPlayer) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(frame.area());

    match app.ui_mode {
        UiMode::Default => render_default(frame, app, player, vertical_chunks[0]),
        UiMode::FullScreenPlayer => render_full_screen(frame, app, player, vertical_chunks[0]),
    }

    render_footer(frame, app, vertical_chunks[1]);
}

fn render_default(frame: &mut Frame, app: &App, player: &MusicPlayer, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(area);

    render_file_list(frame, app, player, chunks[0]);
    render_player_panel(frame, player, chunks[1]);
}

fn render_full_screen(frame: &mut Frame, _app: &App, player: &MusicPlayer, area: Rect) {
    render_player_panel(frame, player, area);
}

fn render_file_list(frame: &mut Frame, app: &App, player: &MusicPlayer, area: Rect) {
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|entry| {
            let icon = if entry.is_dir { "DIR" } else { "MP3" };
            let style = if player.is_playing_track(&entry.path) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(format!("{icon} {}", entry.name)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(app.current_path.to_string_lossy())
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_player_panel(frame: &mut Frame, player: &MusicPlayer, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    render_cava(frame, player, chunks[0]);
    render_song_name(frame, player, chunks[1]);
    render_progress(frame, player, chunks[2]);
}

fn render_cava(frame: &mut Frame, player: &MusicPlayer, area: Rect) {
    let block = Block::default().title("Cava").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let elapsed_ms = player
        .current_position()
        .unwrap_or(Duration::ZERO)
        .as_millis() as f32;
    let phase = elapsed_ms / 120.0;
    let mut line = String::with_capacity(inner.width as usize);
    let levels = [' ', '.', ':', '-', '=', '+', '*', '#'];
    let active = player.state == PlaybackState::Playing;

    for x in 0..inner.width {
        let t = x as f32 * 0.26 + phase;
        let wave = if active {
            ((t.sin() + (t * 0.53 + 1.3).sin() + 2.0) / 4.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let idx = (wave * (levels.len() - 1) as f32).round() as usize;
        line.push(levels[idx]);
    }

    let paragraph = Paragraph::new(line)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(paragraph, inner);
}

fn render_song_name(frame: &mut Frame, player: &MusicPlayer, area: Rect) {
    let block = Block::default().title("Now Playing").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let title = player
        .current_song_name
        .as_deref()
        .unwrap_or("No song playing");
    let state_tag = match player.state {
        PlaybackState::Playing => "PLAY",
        PlaybackState::Paused => "PAUSE",
        PlaybackState::Stopped => "STOP",
    };

    let paragraph = Paragraph::new(format!("[{state_tag}] {title}"))
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(paragraph, inner);
}

fn render_progress(frame: &mut Frame, player: &MusicPlayer, area: Rect) {
    let block = Block::default().title("Progress").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let position = player.current_position().unwrap_or(Duration::ZERO);
    let duration = player.current_duration();
    let elapsed = format_duration(position);

    let (ratio, label) = match duration {
        Some(total) if total.as_secs_f64() > 0.0 => {
            let progress = (position.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0);
            (progress, format!("{elapsed} / {}", format_duration(total)))
        }
        _ => (0.0, format!("{elapsed} / --:--")),
    };

    let gauge = Gauge::default()
        .ratio(ratio)
        .label(label)
        .gauge_style(Style::default().fg(Color::LightBlue).bg(Color::DarkGray));
    frame.render_widget(gauge, inner);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut text = String::from(
        "Up/Down: Navigate | Enter: Open/Play/Pause | 1: Split | 2: Player | Esc: Quit",
    );
    if let Some(status) = &app.status {
        text.push_str(" | ");
        text.push_str(status);
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Keys").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
