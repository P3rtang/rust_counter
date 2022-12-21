use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    layout::{Layout, Constraint, Direction, Rect},
    style::{Style, Color},
    Frame
};
use std::io::Stdout;
use crate::{counter::{Counter, CounterStore}, entry::EntryState};
use crate::app::AppState;
use crate::entry::Entry;

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
                Constraint::Min(20),
                Constraint::Percentage(80),
            ].as_ref()
        )
        .split(f.size());
    let counter_list = List::new(c_list.get_counters().iter().map(|counter| ListItem::new(counter.name())).collect::<Vec<ListItem>>())
        .block(Block::default().title("Counters").borders(Borders::ALL).style(Style::default().fg(BLUE)))
        .style(Style::default().fg(BLUE))
        .highlight_style(Style::default().fg(Color::Magenta))
        .highlight_symbol(" > ");
    let block = Block::default()
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(
        c_list.get(c_list_state.selected().unwrap_or(0))
        .unwrap_or(&Counter::default())
        .get_count()
        .to_string())
        .block(block);

    f.render_stateful_widget(counter_list, chunks[0], c_list_state);
    f.render_widget(paragraph, chunks[1]);
    if state == AppState::AddingNew {
        draw_new_counter(f, entry_state)
    } 
}

pub fn draw_new_counter(f: &mut Frame<CrosstermBackend<Stdout>>, entry_state: &mut EntryState) {
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let entry = Entry::default()
        .style(Style::default().fg(Color::White).bg(GRAY))
        .title("Name new counter")
        .field_width(12)
        .block(block);
    f.render_stateful_widget(entry, size, entry_state);
    if let Some(pos) = entry_state.get_cursor() {
        f.set_cursor(pos.0, pos.1)
    }
}
