use crossterm::event::KeyCode;
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
    layout::{Layout, Constraint, Direction, Rect, Alignment},
    style::{Style, Color, Modifier},
    Frame
};
use std::{io::Stdout, time::Duration};
use crate::{counter::Counter, entry::EntryState};
use crate::app::{App, AppState};
use crate::entry::Entry;
use crate::dialog::Dialog;

const BLUE:      Color = Color::Rgb(139, 233, 253);
const GRAY:      Color = Color::Rgb(100, 114, 125);
const MAGENTA:   Color = Color::Rgb(255, 121, 198);
const DARK_GRAY: Color = Color::Rgb(40, 42, 54);
const GREEN:     Color = Color::Rgb(80, 250, 123);
const ORANGE:    Color = Color::Rgb(255, 184, 108);
const BRIGHT_RED: Color = Color::Rgb(255, 149, 128);
const YELLOW: Color = Color::Rgb(241, 250, 140);


#[derive(PartialEq)]
pub enum UiSize {
    Small,
    Big,
}

pub fn draw(f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
                Constraint::Min(15),
                Constraint::Percentage(80),
            ].as_ref()
        )
        .split(f.size());
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
                Constraint::Length(6),
                Constraint::Min(5),
            ].as_ref()
        )
        .split(chunks[1]);

    if app.ui_size == UiSize::Big && app.get_active_counter().is_some() {
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(4), Constraint::Length(4)].as_ref())
            .split(right_chunks[0]);

        let gauge_block = Block::default()
            .borders(Borders::TOP.complement())
            .style(Style::default().fg(Color::Black));

        let progress = app.get_active_counter().unwrap().get_progress();
        let color: Color;
        if progress < 0.5 { color = GREEN }
        else if app.get_active_counter().unwrap().get_count() < app.get_active_counter().unwrap().get_progress_odds() as i32 { color = YELLOW }
        else if progress < 0.75 { color = ORANGE }
        else { color = BRIGHT_RED }

        let progress_bar = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::Black).add_modifier(Modifier::BOLD))
            .block(gauge_block)
            .ratio(progress)
            .label(format!("{:.3}", progress * 100.0));
        f.render_widget(progress_bar, chunk[1]);
    }

    let mut list_block = Block::default()
        .title("Counters")
        .borders(Borders::ALL)
        .style(Style::default().fg(BLUE));
    let mut paragraph_block = Block::default()
        .borders(Borders::ALL)
        .border_attach(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::White));
    let time_block = Block::default()
        .borders(Borders::TOP.complement());

    if app.app_state == AppState::Counting {
        list_block = list_block.style(Style::default().fg(Color::White)).title("");
        paragraph_block = paragraph_block
            .border_style(Style::default().fg(BLUE))
            .title(format!("{}", app.c_store.get(app.c_state.selected().unwrap_or(0)).unwrap().get_name()));
    }

    let counter_list = List::new(
            app.c_store.get_counters()
            .iter()
            .map(|counter| ListItem::new(counter.get_name()))
            .collect::<Vec<ListItem>>()
        )
        .block(list_block)
        .highlight_style(Style::default().fg(MAGENTA))
        .highlight_symbol(" > ");

    let paragraph = Paragraph::new(
            format_paragraph(
                app.c_store.get(
                    app.c_state.selected().unwrap_or(0))
                    .unwrap_or(&Counter::default())
                .get_count()
                .to_string()
            )
        )
        .block(paragraph_block)
        .alignment(Alignment::Center);

    let paragraph_time = Paragraph::new(
            format_paragraph(
                format_duration(
                    app.c_store.get(app.c_state.selected().unwrap_or(0))
                    .unwrap_or(&Counter::default())
                    .get_time(),
                    app.time_show_millis
                )
            )
        )
        .block(time_block)
        .alignment(Alignment::Center);

    f.render_stateful_widget(counter_list, chunks[0], &mut app.c_state);
    f.render_widget(paragraph, right_chunks[0]);
    f.render_widget(paragraph_time, right_chunks[1]);

    if app.app_state == AppState::AddingNew {
        draw_entry(f, &mut app.entry_state, "Name new Counter", (50, 10))
    } else if app.app_state == AppState::Rename || app.app_state == AppState::Editing(0) {
        draw_entry(f, &mut app.entry_state, "Rename Counter", (50, 10))
    } else if app.app_state == AppState::ChangeCount || app.app_state == AppState::Editing(1) {
        draw_entry(f, &mut app.entry_state, "Change Count", (50, 10))
    } else if app.app_state == AppState::Editing(2) {
        draw_entry(f, &mut app.entry_state, "Change Time", (50, 10))
    } else if app.app_state == AppState::Delete {
        let name = app.c_store.get(app.c_state.selected().unwrap_or(0)).unwrap().get_name();
        draw_delete_dialog(f, &name)
    }
}

// format any time to a readable digital clock with hours as the highest divider
fn format_duration(duration: Duration, show_millis: bool) -> String {
    let millis = duration.as_millis();
    let secs   = millis / 1000;
    let mins   = secs / 60;
    let hours  = mins / 60;
    if show_millis {
        return format!("{:02}:{:02}:{:02},{:03}", hours, mins % 60, secs % 60, millis % 1000)
    }
    return format!("{:02}:{:02}:{:02},***", hours, mins % 60, secs % 60)
}

fn format_paragraph(mut text: String) -> String {
    text.insert(0, '\n');
    text
}

fn draw_entry(f: &mut Frame<CrosstermBackend<Stdout>>, entry_state: &mut EntryState, title: &str, size: (u16, u16)) {
    let mut window = f.size();
    if window.width >= size.0 && window.height >= size.1 {
        window = Rect::new((window.right() - size.0) / 2, (window.bottom() - size.1) / 2, size.0, size.1);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let entry = Entry::default()
        .title(title)
        .field_width(12)
        .style(Style::default().fg(BLUE).bg(GRAY))
        .field_style(Style::default().fg(BLUE))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_stateful_widget(entry, window, entry_state);
    if let Some(pos) = entry_state.get_cursor() {
        f.set_cursor(pos.0, pos.1)
    }
}

fn draw_delete_dialog(f: &mut Frame<CrosstermBackend<Stdout>>, name: &str) {
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let dialog = Dialog::default()
        .message(&format!("Are you sure\nyou want to delete {}?", name))
        .style(Style::default().fg(Color::Red).bg(GRAY))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_widget(dialog, size);
}
