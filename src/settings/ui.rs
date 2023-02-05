use crate::app::AppError;
use std::io::Stdout;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, TableState},
    Frame,
};

use super::{item::MainContents, ContentKey};

pub const BLUE: Color = Color::Rgb(139, 233, 253);
pub const GRAY: Color = Color::Rgb(100, 114, 125);
pub const MAGENTA: Color = Color::Rgb(255, 121, 198);
pub const BACKGROUND: Color = Color::Rgb(40, 42, 54);
pub const GREEN: Color = Color::Rgb(80, 250, 123);
pub const ORANGE: Color = Color::Rgb(255, 184, 108);
pub const BRIGHT_RED: Color = Color::Rgb(255, 149, 128);
pub const YELLOW: Color = Color::Rgb(241, 250, 140);
pub const BORDER: Color = Color::Rgb(100, 114, 125);

#[derive(Default, Clone)]
pub enum WindowState {
    #[default]
    Default,
    SubMenu(ContentKey),
}

#[derive(Default)]
pub struct SettingsWindow {
    state: WindowState,
    table_state: TableState,
    style: Style,
    highl_style: Style,
    border_style: Style,
    layout: Vec<Constraint>,
}

impl SettingsWindow {
    pub fn new() -> Self {
        Self {
            state: WindowState::default(),
            table_state: TableState::default(),
            style: Style::default().bg(BACKGROUND),
            highl_style: Style::default().fg(BACKGROUND).bg(BORDER),
            border_style: Style::default().fg(BORDER).bg(BACKGROUND),
            layout: vec![Constraint::Percentage(40), Constraint::Percentage(60)],
        }
    }

    pub fn draw(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        setting_items: &MainContents,
    ) -> Result<(), AppError> {
        if area.width < 40 || area.height < 10 {
            return Err(AppError::ScreenSize(format!(
                "Screen size too small should be 40x10 minumum got {}x{}",
                area.width, area.height
            )));
        }

        f.render_widget(Clear, area);

        // let split = Layout::default()
        //     .direction(tui::layout::Direction::Horizontal)
        //     .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        //     .split(area);

        let border_style = Style::default().fg(BORDER).bg(BACKGROUND);

        let border_block = Block::default()
            .borders(Borders::ALL)
            .style(self.style)
            .border_style(border_style)
            .border_type(BorderType::Double);
        f.render_widget(border_block, area);

        self.draw_submenu(
            &setting_items,
            f,
            area.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }),
        )?;

        Ok(())
    }

    pub fn select_next(&mut self, list_len: usize) {
        self.table_state.select(Some(
            self.table_state
                .selected()
                .map_or(0, |num| (num + 1) % list_len),
        ));
    }
    pub fn select_prev(&mut self, list_len: usize) {
        self.table_state.select(Some(
            self.table_state
                .selected()
                .map_or(2, |num| (list_len + num - 1) % list_len),
        ));
    }

    pub fn set_state(&mut self, state: WindowState) {
        self.state = state
    }
    pub fn get_state(&self) -> WindowState {
        return self.state.clone();
    }

    pub fn get_selected_key(&self) -> Result<ContentKey, AppError> {
        match self.table_state.selected().unwrap_or(0) {
            0 => Ok(ContentKey::TickRate),
            1 => Ok(ContentKey::ShowMillis),
            2 => Ok(ContentKey::ActKeyboard),
            _ => return Err(AppError::ImpossibleState("Settings Main List".to_string())),
        }
    }

    pub fn draw_submenu<'a>(
        &'a self,
        setting_items: &'a MainContents,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
    ) -> Result<(), AppError> {
        let table = setting_items
            .get_active_table()
            .style(self.style)
            .highlight_style(self.highl_style)
            .widths(&self.layout);
        f.render_stateful_widget(table, area, &self.table_state);

        match &self.state {
            WindowState::Default => {}
            WindowState::SubMenu(key) => {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(self.layout.clone())
                    .split(area);
                f.render_widget(Clear, split[1]);

                setting_items.draw_item(f, key, split[1])?;
            }
        }
        Ok(())
    }
}
