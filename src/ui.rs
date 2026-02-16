use std::rc::Rc;
use crate::player::MusicPlayer;
use crate::app::App;
use std::time::Duration;
use color_eyre::{eyre::{Ok,Result}};
use crossterm::event::MouseButton;
use ratatui::{
    DefaultTerminal, Frame, buffer::Buffer, widgets::Gauge, crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind}, layout::{Constraint, Direction, Layout, Rect, Alignment}, macros::vertical, style::{
        Color, Modifier, Style, Stylize, palette::tailwind::{BLUE, GREEN, SLATE}
    }, symbols, text::Line, widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
        StatefulWidget, Widget, Wrap,
    }
};


pub fn render(frame:&mut Frame, app: &mut App, music_player: &mut MusicPlayer){
    let mut vertical_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Min(0),
        Constraint::Length(3), 
    ])
    .split(frame.area());

    render_info_keybind(frame, app, vertical_chunks[1]);

    match app.ui_mode {
        crate::app::UImode::Default => {render_default(frame, app, music_player, &mut vertical_chunks);}
        crate::app::UImode::Full_screen_player => {render_full_screen_player(frame, app,music_player, &mut vertical_chunks);}
    };
}

fn render_default(frame:&mut Frame, app: &mut App, music_player: &mut MusicPlayer, chunks_vertical: &mut Rc<[Rect]>){
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks_vertical[0]);

    render_file_list(frame, app, chunks[0]);
    render_info_panel(frame, app, music_player, chunks[1]);
}

fn render_full_screen_player(frame:&mut Frame, app: &mut App, music_player: &mut MusicPlayer, chunks_vertical: &mut Rc<[Rect]>){
    let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(100)])
    .split(chunks_vertical[0]);

    render_info_panel(frame, app, music_player, chunks[0]);
}


fn render_file_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.archieves
    .iter()
    .map(|entry| {
        let is_current = Some(&entry.path) == app.current_song_path.as_ref();

        let style = if is_current {
                Style::default().fg(Color::LightYellow)
            } else {
                Style::default()
            };


        let icon = if entry.is_dir { "🖿 " } else { "♫ " };
        ListItem::new(format!("{} {}", icon, entry.name)).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().title(app.current_path.to_string_lossy()).borders(Borders::ALL))
        .highlight_style(Style::new().fg(Color::LightYellow).bg(Color::DarkGray))
        .highlight_symbol("-> ");

    let mut list_state = ListState::default();
    list_state.select(Some(app.select_index));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_info_panel(frame: &mut Frame, app: &App, music_player: &mut MusicPlayer, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    //Top: Cava (simple bar visualizer)
    render_cava(frame, app,music_player, chunks[0]);

    //Middle: Song name (without extension)
    render_song_name(frame, app, chunks[1]);

    //Bottom: Progress bar and time
    render_progress_bar(frame, app,music_player, chunks[2]);
}

fn render_cava(frame: &mut Frame, app: &App, music_player: &mut MusicPlayer, area: Rect) {
    let block = Block::default().title("Cava").borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.width < 10 || inner_area.height < 1 {
        return;
    }

    // Generate some fake bars based on the current playback position
    let bars_count = inner_area.width as usize;
    let max_height = inner_area.height as usize;

    // Use a simple hash of the current time to produce pseudo-random heights
    let seed = MusicPlayer::current_position(music_player).unwrap_or(Duration::ZERO).as_millis() as usize;
    let mut bars = Vec::with_capacity(bars_count);

    for i in 0..bars_count {
        // Simple pseudo-random: (seed + i * 13) % (max_height+1)
        let height = ((seed.wrapping_add(i * 13)) % (max_height + 1)) as u16;
        bars.push(height);
    }

    // Draw each bar as a vertical line of blocks
    for (x, &bar_height) in bars.iter().enumerate() {
        for y in 0..bar_height {
            let y_pos = inner_area.bottom() - 1 - y;
            let cell = frame.buffer_mut().get_mut(inner_area.left() + x as u16, y_pos);
            cell.set_char('█');
            cell.set_fg(Color::LightMagenta);
        }
    }
}

/// Renders the name of the current song (without extension) or a placeholder.
fn render_song_name(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().title("Now Playing").borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let song_name = if let Some(path) = &app.current_song_path {
        // Remove extension
        if let Some(stem) = path.file_stem() {
            stem.to_string_lossy().to_string()
        } else {
            "Unknown".to_string()
        }
    } else {
        "No song playing".to_string()
    };

    let paragraph = Paragraph::new(song_name)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    frame.render_widget(paragraph, inner_area);
}

/// Renders a progress bar with current/total time.
fn render_progress_bar(frame: &mut Frame, app: &App, music_player: &MusicPlayer, area: Rect) {
    let block = Block::default().title("Progress").borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let (percent, label) = if let (Some(pos), Some(dur)) = (music_player.current_position(), music_player.current_duration()) {
        let pos_secs = pos.as_secs();
        let dur_secs = dur.as_secs();
        let percent = if dur_secs > 0 {
            (pos_secs as f64 / dur_secs as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let label = format!(
            "{:02}:{:02} / {:02}:{:02}",
            pos_secs / 60, pos_secs % 60,
            dur_secs / 60, dur_secs % 60
        );
        (percent, label)
    } else {
        (0.0, "00:00 / 00:00".to_string())
    };

    let gauge = Gauge::default()
        .ratio(percent)
        .label(label)
        .style(Style::default().fg(Color::LightBlue))
        .gauge_style(Style::default().fg(Color::LightBlue).bg(Color::DarkGray));
    frame.render_widget(gauge, inner_area);
}

fn render_info_keybind(frame: &mut Frame, app: &App, area: Rect){
    let text: String = "↑ ↓: navegation | Enter: select | Esc: quit | 1,2: Interface".to_string();

    let paragraph: Paragraph = Paragraph::new(text)
    .block(Block::default().title("binds").borders(Borders::ALL))
    .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}