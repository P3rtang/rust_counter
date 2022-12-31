use crossterm::event::KeyCode;
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
    layout::{Layout, Constraint, Direction, Rect, Alignment},
    style::{Style, Color, Modifier},
    Frame
};
use std::{io::Stdout, time::Duration};
use crate::entry::EntryState;
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
        let counter = app.get_unsafe_counter();
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(4), Constraint::Length(4)].as_ref())
            .split(right_chunks[0]);

        let gauge_block = Block::default()
            .borders(Borders::TOP.complement())
            .style(Style::default().fg(Color::Black));

        let progress = counter.get_progress();
        let color: Color;
        if progress < 0.5 { color = GREEN }
        else if counter.get_count() < counter.get_progress_odds() as i32 { color = YELLOW }
        else if progress < 0.75 { color = ORANGE }
        else { color = BRIGHT_RED }

        let progress_bar = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::Black).add_modifier(Modifier::BOLD))
            .block(gauge_block)
            .ratio(progress)
            .label(format!("{:.3}", progress * 100.0));
        f.render_widget(progress_bar, chunk[1]);
    }

    let state = app.app_state.clone();

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

    if let AppState::Counting(num) = app.app_state {
        list_block = list_block.style(Style::default().fg(Color::White)).title("");
        let title = match num {
            0 => app.get_unsafe_counter().get_name(),
            1 => app.get_unsafe_counter().get_phase_name(app.phase_list_state.selected().unwrap_or(0)),
            _ => "".to_string()
        };
        paragraph_block = paragraph_block
            .border_style(Style::default().fg(BLUE))
            .title(title);
    }

    let mut list = app.c_store
        .get_counters()
        .iter()
        .map(|counter| ListItem::new(counter.borrow().get_name()))
        .collect::<Vec<ListItem>>();

    let mut active_count = app.get_active_counter().map(|c| c.get_count()).unwrap_or(0);
    let mut active_time  = app.get_active_counter().map(|c| c.get_time()).unwrap_or(Duration::default());
    let mut list_state   = &mut app.c_state;

    if  state == AppState::PhaseSelect ||
        state == AppState::RenamePhase || 
        state == AppState::Counting(1)  
    {
        list = app.get_active_counter().unwrap()
            .get_phases().iter()
            .map(|phase| ListItem::new(phase.get_name()))
            .collect();

        active_count = app.get_unsafe_counter().get_nphase_count(app.phase_list_state.selected().unwrap_or(0));
        active_time  = app.get_unsafe_counter().get_nphase_time(app.phase_list_state.selected().unwrap_or(0));
        list_state   = &mut app.phase_list_state
    }

    let counter_list = create_list(list, list_block);

    let paragraph = Paragraph::new(format_paragraph(active_count.to_string()))
        .block(paragraph_block)
        .alignment(Alignment::Center);

    let paragraph_time = Paragraph::new(
            format_paragraph(
                format_duration(active_time, app.time_show_millis)
            )
        )
        .block(time_block)
        .alignment(Alignment::Center);

    f.render_stateful_widget(counter_list, chunks[0], list_state);
    f.render_widget(paragraph, right_chunks[0]);
    f.render_widget(paragraph_time, right_chunks[1]);

    // if any the app is in an entry state draw them last so they go on top
    match app.app_state {
        AppState::AddingNew => {
            draw_entry(f, &mut app.entry_state, "Name new Counter", (50, 10))
        }
        AppState::RenamePhase => {
            let phase_title = format!("give phase {}\n a name", app.get_unsafe_counter().get_phase_name(app.phase_list_state.selected().unwrap_or(0)));
            draw_entry(f, &mut app.entry_state, &phase_title, (50, 10));
        }
        AppState::Rename | AppState::Editing(0) => { 
            draw_entry(f, &mut app.entry_state, "Change Name", (50, 10)) 
        }
        AppState::ChangeCount | AppState::Editing(1) => {
            draw_entry(f, &mut app.entry_state, "Change Count", (50, 10))
        }
        AppState::Editing(2) => {
            draw_entry(f, &mut app.entry_state, "Change Time", (50, 10));
        }
        AppState::Delete => {
            let name = app.get_active_counter().unwrap().get_name();
            draw_delete_dialog(f, &name)
        }
        AppState::DeletePhase =>  {
            if app.get_unsafe_counter().get_phase_count() > 1 {
                let name = app.get_active_counter().unwrap().get_phase_name(app.c_state.selected().unwrap_or(1));
                draw_delete_dialog(f, &name)
            }
        }
        _ => {}
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

fn create_list<'a>(list: Vec<ListItem<'a>>, block: Block<'a>) -> List<'a> {
    let counter_list = List::new(list)
        .block(block)
        .highlight_style(Style::default().fg(MAGENTA))
        .highlight_symbol(" > ");
    counter_list
}
