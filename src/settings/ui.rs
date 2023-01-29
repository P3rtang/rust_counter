use crate::app::AppError;
use std::io::Stdout;
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};

const BLUE: Color = Color::Rgb(139, 233, 253);
const GRAY: Color = Color::Rgb(100, 114, 125);
const MAGENTA: Color = Color::Rgb(255, 121, 198);
const BACKGROUND: Color = Color::Rgb(40, 42, 54);
const GREEN: Color = Color::Rgb(80, 250, 123);
const ORANGE: Color = Color::Rgb(255, 184, 108);
const BRIGHT_RED: Color = Color::Rgb(255, 149, 128);
const YELLOW: Color = Color::Rgb(241, 250, 140);
const BORDER: Color = Color::Rgb(100, 114, 125);

#[derive(Default)]
enum WindowState {
    #[default]
    Default,
}

#[derive(Default)]
pub struct SettingsWindow {
    state: WindowState,
    stngs_list: ListState,
}

impl SettingsWindow {
    pub fn new() -> Self {
        Self {
            state: WindowState::default(),
            stngs_list: ListState::default(),
        }
    }

    pub fn draw(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
    ) -> Result<(), AppError> {
        if area.width < 40 || area.height < 10 {
            return Err(AppError::ScreenSize(format!(
                "Screen size too small should be 40x10 minumum got {}x{}",
                area.width, area.height
            )));
        }

        f.render_widget(Clear, area);

        let list_items = [
            ListItem::new("active keyboard"),
            ListItem::new("tick rate"),
            ListItem::new("show milliseconds"),
        ];

        let style = Style::default().bg(BACKGROUND);
        let highl_style = Style::default().fg(BACKGROUND).bg(BORDER);

        let border_style = Style::default().fg(BORDER);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default())
            .border_style(border_style)
            .border_type(BorderType::Double);

        let list_widget = List::new(list_items)
            .block(block)
            .style(style)
            .highlight_style(highl_style);

        f.render_stateful_widget(list_widget, area, &self.stngs_list);
        Ok(())
    }

    pub fn select_next(&mut self) {
        self.stngs_list.select(Some(
            self.stngs_list.selected().map_or(0, |num| (num + 1) % 3),
        ));
    }
    pub fn select_prev(&mut self) {
        self.stngs_list.select(Some(
            self.stngs_list
                .selected()
                .map_or(2, |num| (3 + num - 1) % 3),
        ));
    }
}
