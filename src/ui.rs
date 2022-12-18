use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    layout::{Layout, Constraint, Direction, Rect},
    style::{Style, Color},
    Frame
};
use std::io::Stdout;
use crate::counter::{Counter, CounterStore};

const BLUE: Color = Color::Rgb(139, 233, 253);

pub fn draw(f: &mut Frame<CrosstermBackend<Stdout>>, c_list: &CounterStore, c_list_state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
                Constraint::Percentage(20),
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
}
