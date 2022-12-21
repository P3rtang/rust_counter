use crossterm::event::KeyCode;
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    layout::{Layout, Constraint, Direction, Rect},
    style::{Style, Color},
    Frame
};
use std::{io::Stdout, time::Duration};
use crate::{counter::{Counter, CounterStore}, entry::EntryState};
use crate::app::AppState;
use crate::entry::Entry;
use crate::dialog::Dialog;

const BLUE: Color = Color::Rgb(139, 233, 253);
const GRAY: Color = Color::Rgb(100, 114, 125);

pub fn draw(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    c_list: &CounterStore,
    c_list_state: &mut ListState,
    state: AppState,
    entry_state: &mut EntryState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
                Constraint::Min(15),
                Constraint::Percentage(80),
            ].as_ref()
        )
        .split(f.size());
    let righ_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ].as_ref()
        )
        .split(chunks[1]);

    let mut list_block = Block::default()
        .title("Counters")
        .borders(Borders::ALL)
        .style(Style::default().fg(BLUE));
    let mut paragraph_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));
    let time_block = Block::default()
        .borders(Borders::TOP.complement());

    if state == AppState::Counting {
        list_block = list_block.style(Style::default().fg(Color::White)).title("");
        paragraph_block = paragraph_block
            .style(Style::default().fg(BLUE))
            .title(format!("{}", c_list.get(c_list_state.selected().unwrap_or(0)).unwrap().get_name()));
    }

    let counter_list = List::new(
            c_list.get_counters()
            .iter()
            .map(|counter| ListItem::new(counter.get_name()))
            .collect::<Vec<ListItem>>()
        )
        .block(list_block)
        .highlight_style(Style::default().fg(Color::Magenta))
        .highlight_symbol(" > ");

    let paragraph = Paragraph::new(
            format_paragraph(
                c_list.get(
                    c_list_state.selected().unwrap_or(0))
                    .unwrap_or(&Counter::default())
                .get_count()
                .to_string()
            )
        )
        .block(paragraph_block)
        .style(Style::default().fg(Color::White));

    let paragraph_time = Paragraph::new(
            format_paragraph(
                format_duration(
                    c_list.get(c_list_state.selected().unwrap_or(0))
                    .unwrap_or(&Counter::default())
                    .get_time()
                )
            )
        )
        .block(time_block);

    f.render_stateful_widget(counter_list, chunks[0], c_list_state);
    f.render_widget(paragraph, righ_chunks[0]);
    f.render_widget(paragraph_time, righ_chunks[1]);

    if state == AppState::AddingNew {
        draw_new_counter(f, entry_state)
    } else if state == AppState::Rename || state == AppState::Editing(0) {
        draw_rename(f, entry_state)
    } else if state == AppState::ChangeCount || state == AppState::Editing(1) {
        draw_change_count(f, entry_state)
    } else if state == AppState::Editing(2) {
        draw_change_time(f, entry_state)
    } else if state == AppState::Delete {
        let name = c_list.get(c_list_state.selected().unwrap_or(0)).unwrap().get_name();
        draw_delete_dialog(f, &name)
    }
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    let secs   = millis / 1000;
    let mins   = secs / 60;
    let hours  = mins / 60;
    return format!("{:02}:{:02}:{:02},{:03}", hours, mins % 60, secs % 60, millis % 1000)
}

fn format_paragraph(text: String) -> String {
    let mut return_str = String::new();
    return_str.push_str("\n  ");
    return_str.push_str(&text);
    return_str
}

fn draw_new_counter(f: &mut Frame<CrosstermBackend<Stdout>>, entry_state: &mut EntryState) {
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let entry = Entry::default()
        .title("Name new counter")
        .field_width(12)
        .style(Style::default().fg(BLUE).bg(GRAY))
        .field_style(Style::default().fg(BLUE))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_stateful_widget(entry, size, entry_state);
    if let Some(pos) = entry_state.get_cursor() {
        f.set_cursor(pos.0, pos.1)
    }
}

fn draw_rename(f: &mut Frame<CrosstermBackend<Stdout>>, entry_state: &mut EntryState) {
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let entry = Entry::default()
        .title("Rename counter")
        .field_width(12)
        .style(Style::default().fg(BLUE).bg(GRAY))
        .field_style(Style::default().fg(BLUE))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_stateful_widget(entry, size, entry_state);
    if let Some(pos) = entry_state.get_cursor() {
        f.set_cursor(pos.0, pos.1)
    }
}

fn draw_change_count(f: &mut Frame<CrosstermBackend<Stdout>>, entry_state: &mut EntryState) {
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let entry = Entry::default()
        .title("Change counter amount")
        .field_width(12)
        .style(Style::default().fg(BLUE).bg(GRAY))
        .field_style(Style::default().fg(BLUE))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_stateful_widget(entry, size, entry_state);
    if let Some(pos) = entry_state.get_cursor() {
        f.set_cursor(pos.0, pos.1)
    }
}

fn draw_change_time(f: &mut Frame<CrosstermBackend<Stdout>>, entry_state: &mut EntryState) {
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let entry = Entry::default()
        .title("Set Counter time (min)")
        .field_width(12)
        .style(Style::default().fg(BLUE).bg(GRAY))
        .field_style(Style::default().fg(BLUE))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_stateful_widget(entry, size, entry_state);
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
        .title(&format!("Are you sure you want to delete {}?", name))
        .style(Style::default().fg(Color::Red).bg(GRAY))
        .keys(KeyCode::Esc, KeyCode::Enter)
        .block(block);
    f.render_widget(dialog, size);
}
