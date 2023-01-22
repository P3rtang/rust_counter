use crossterm::event::KeyCode;
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
    layout::{Layout, Constraint, Direction, Rect, Alignment},
    style::{Style, Color, Modifier},
    Frame
};
use std::{io::Stdout, time::Duration};
use crate::app::{App, AppMode, DialogState as DS, EditingState as ES, AppError};
use crate::widgets::entry::{Entry, EntryState};
use crate::widgets::dialog::Dialog;

const BLUE:       Color = Color::Rgb(139, 233, 253);
const GRAY:       Color = Color::Rgb(100, 114, 125);
const MAGENTA:    Color = Color::Rgb(255, 121, 198);
const DARK_GRAY:  Color = Color::Rgb( 40,  42,  54);
const GREEN:      Color = Color::Rgb( 80, 250, 123);
const ORANGE:     Color = Color::Rgb(255, 184, 108);
const BRIGHT_RED: Color = Color::Rgb(255, 149, 128);
const YELLOW:     Color = Color::Rgb(241, 250, 140);

// TODO: remove this enum
#[derive(PartialEq, Eq)]
pub enum UiWidth {
    Compact,
    Small,
    Medium,
    Big,
}

pub fn draw(f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App) -> Result<(), AppError> {
    app.ui_size = match f.size().width {
        0..=27 => UiWidth::Small,
        28..=60 => UiWidth::Medium,
        _ => UiWidth::Big,
    };
    let constraints = match app.ui_size {
        UiWidth::Medium => { 
            vec![Constraint::Min(15), Constraint::Percentage(80)]
        }
        UiWidth::Big => {
            vec![Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(60)]
        }
        UiWidth::Compact => {
            vec![Constraint::Percentage(100)]
        }
        UiWidth::Small => {
            vec![Constraint::Percentage(100)]
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(f.size());

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
                Constraint::Length(6),
                Constraint::Min(5),
            ].as_ref()
        )
        .split(chunks[chunks.len() - 1]);


    draw_counter_list(f, app, chunks[0]);
    draw_phase_list(f, app, &chunks);
    draw_text_boxes(f, app, &right_chunks)?;
    draw_progress_gauge(f, app, &right_chunks)?;

    // if any the app is in an entry state draw them last so they go on top
    match app.get_mode() {
        AppMode::Selection(DS::AddNew) => {
            draw_entry(f, app.get_entry_state(0), "Name new Counter", (50, 10))
        }
        AppMode::PhaseSelect(DS::Editing(_)) => {
            let phase_title = format!(
                "give phase {}\n a name",
                app.get_act_phase_name()?
            );
            draw_entry(f, app.get_entry_state(0), phase_title, (50, 10));
        }
        AppMode::Selection(DS::Editing(ES::Rename(_))) => {
            draw_entry(f, app.get_entry_state(0), "Change Name", (50, 10)) 
        }
        AppMode::Selection(DS::Editing(ES::ChCount(_))) => {
            draw_entry(f, app.get_entry_state(0), "Change Count", (50, 10))
        }
        AppMode::Selection(DS::Editing(ES::ChTime(_))) => {
            draw_entry(f, app.get_entry_state(0), "Change Time", (50, 10));
        }
        AppMode::Selection(DS::Delete) => {
            let name = app.get_act_counter()?.get_name();
            draw_delete_dialog(f, name)
        }
        AppMode::PhaseSelect(DS::Delete) =>  {
            if app.get_act_counter()?.get_phase_count() > 1 {
                let name = app.get_act_phase_name()?;
                draw_delete_dialog(f, name)
            }
        }
        _ => {}
    }
    Ok(())
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
    format!("{:02}:{:02}:{:02},***", hours, mins % 60, secs % 60)
}

fn format_paragraph(text: impl Into<String>) -> String {
    let mut text = text.into();
    text.insert(0, '\n');
    text
}

fn draw_entry(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    entry_state: &mut EntryState,
    title: impl Into<String>,
    size: (u16, u16)
) {
    let mut window = f.size();
    if window.width >= size.0 && window.height >= size.1 {
        window = Rect::new(
            (window.right() - size.0) / 2,
            (window.bottom() - size.1) / 2,
            size.0,
            size.1
        );
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

    let pos_val = entry_state.get_cursor().map(|cur| cur);
    if let Some(pos) = pos_val {
        f.set_cursor(pos.0, pos.1)
    }
}

fn draw_delete_dialog
    (f: &mut Frame<CrosstermBackend<Stdout>>, name: impl Into<String> + std::fmt::Display)
{
    let mut size = f.size();
    if size.width >= 50 && size.height >= 10 {
        size = Rect::new((size.right() - 50) / 2, (size.bottom() - 10) / 2, 50, 10);
    }
    let block = Block::default()
        .borders(Borders::ALL);
    let dialog = Dialog::default()
        .message(format!("Are you sure\nyou want to delete {name}?"))
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

fn draw_counter_list(f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App, area: Rect) {
    // if the app uisize is small hide the main counter list when phases are displayed
    // if the list is displayed it should be blue when it is the active widget
    use AppMode::*;

    let (color, title) = match app.get_mode() {
        PhaseSelect(_) | Counting(1) | KeyLogger(1)
            if app.ui_size == UiWidth::Small || app.ui_size == UiWidth::Compact => return,
        PhaseSelect(_) | Counting(_) | KeyLogger(_) => (Color::White, ""),
        _ => (BLUE, "Counters"),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(color));
    let list_widget = create_list(
        app.c_store
            .get_counters()
            .iter()
            .map(|c| ListItem::new(c.borrow().get_name()))
            .collect(),
        block
    );
    f.render_stateful_widget(list_widget, area, app.get_list_state(0))
}

fn draw_phase_list(f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App, area: &[Rect]) {
    use AppMode::*;
    
    let (color, title) = match app.get_mode() {
        Selection(_) | Counting(0) | KeyLogger(0)
            if app.ui_size == UiWidth::Small || app.ui_size == UiWidth::Compact => return,
        Selection(_) | Counting(_) | KeyLogger(_) => (Color::White, ""),
        _ => (BLUE, "Phases")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(color));

    let mut rect_ind = 1;
    if app.ui_size == UiWidth::Compact || app.ui_size == UiWidth::Small { rect_ind = 0 }

    let list_widget = if let Ok(counter) = app.get_act_counter() {
        create_list(
            counter
                .get_phases()
                .iter()
                .map(|p| ListItem::new(p.get_name()))
                .collect(),
            block
        )
    } else {
        create_list(vec![], block)
    };
    f.render_stateful_widget(list_widget, area[rect_ind], app.get_list_state(1))
}

fn draw_text_boxes
    (f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App, area: &[Rect])
    -> Result<(), AppError> 
{
    use AppMode::*;
    let (color, title) = match app.get_mode() {
        Counting(_) => (BLUE, format!(
            "{}-{}",
            app.get_act_counter()?.get_name(),
            app.get_act_phase_name()?
        )),
        KeyLogger(_) => (ORANGE, format!("Keylogger {}", app.get_act_phase_name()?)),
        _ if app.ui_size == UiWidth::Compact || app.ui_size == UiWidth::Small => return Ok(()),
        _ => (Color::White, "".to_string())
    };

    let (active_count, active_time) = match app.get_mode() {
        PhaseSelect(_) | Counting(1) | KeyLogger(1) => (
            app.get_act_phase_count()?,
            app.get_act_phase_time()?,
        ),
        _ => (
            app.get_act_counter().map_or(0,                   |c| c.get_count()),
            app.get_act_counter().map_or(Duration::default(), |c| c.get_time()),
        )
    };

    let paragraph_block = Block::default()
        .borders(Borders::ALL)
        .border_attach(Borders::BOTTOM)
        .title(title)
        .border_style(Style::default().fg(color));
    let time_block = Block::default()
        .borders(Borders::TOP.complement());

    let paragraph = Paragraph::new(format_paragraph(active_count.to_string()))
        .block(paragraph_block)
        .alignment(Alignment::Center);

    let paragraph_time = Paragraph::new(
            format_paragraph(format_duration(active_time, app.time_show_millis))
        )
        .block(time_block)
        .alignment(Alignment::Center);
    if f.size().height >= 10 {
        f.render_widget(paragraph, area[0]);
        f.render_widget(paragraph_time, area[1]);
    } else {
        f.render_widget(paragraph, area[0].union(area[1]))
    }
    Ok(())
}

fn draw_progress_gauge
    (f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App, area: &[Rect])
    -> Result<(), AppError> 
{
    use AppMode::*;

    let progress = app.get_act_counter().map_or(0.0, |c| c.get_progress());
    match app.get_mode() {
        Counting(_) | KeyLogger(_) if app.ui_size == UiWidth::Compact || app.ui_size == UiWidth::Small => {}
        _ if app.ui_size == UiWidth::Compact || app.ui_size == UiWidth::Small => return Ok(()),
        _ => {}
    }
    
    let mut color = GREEN;
    if progress < 0.5 {}
    else if app.get_act_counter()?.get_count() 
        < app.get_act_counter()?.get_progress_odds() as i32 { color = YELLOW }
    else if progress < 0.75 { color = ORANGE }
    else { color = BRIGHT_RED }

    let chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(4)].as_ref())
        .split(area[0]);

    let progress_bar = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(Color::Black).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::complement(Borders::TOP)).border_attach(Borders::BOTTOM))
        .ratio(progress)
        .label(format!("{:.3}", progress * 100.0));
    f.render_widget(progress_bar, chunk[1]);
    Ok(())
}
