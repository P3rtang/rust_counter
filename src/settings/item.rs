use super::ui;
use super::{events::WindowEvent, ContentItemType};
use crate::{
    app::AppError,
    input::{get_kbd_inputs, Event, Key},
    widgets::entry::{Entry, EntryState},
};
use indexmap::IndexMap;
use std::{io::Stdout, marker::PhantomData};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Row, Table},
    Frame,
};

pub trait ToListItem {
    fn to_listitem(&self) -> ListItem;
}

pub trait ToList {
    fn to_list(&self) -> Result<List, AppError>;
}

pub trait ToEntry {
    fn to_entry(&self) -> Result<Entry, AppError>;
}

pub trait SettingsItem: ToListItem + ToList + ToEntry + ToString {
    fn get_key(&self) -> ContentKey;
    fn to_bool(&self) -> Result<bool, AppError>;
    fn to_value(&self) -> Result<u32, AppError>;

    fn handle_event(&mut self, event: Event) -> WindowEvent;
    fn draw(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        style: Style,
        highl_style: Style,
        border_style: Style,
    ) -> Result<(), AppError>;
}

#[derive(Clone, PartialEq, Eq, Hash, Copy)]
pub enum ContentKey {
    TickRate,
    ShowMillis,
    ActKeyboard,
}

impl ContentKey {
    fn to_content_item(self) -> Box<dyn SettingsItem> {
        let kbds = get_kbd_inputs().map(|item| item.clone()).unwrap_or(vec![]);
        return match self {
            ContentKey::TickRate => Box::new(ContentItem::<u32>::new(self, 30, (1, 100))),
            ContentKey::ShowMillis => Box::new(ContentItem::<bool>::new(self, true)),
            ContentKey::ActKeyboard => Box::new(ContentItem::<String>::new(self, kbds)),
        };
    }
}

impl std::fmt::Display for ContentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str_ = match self {
            ContentKey::TickRate => "TickRate",
            ContentKey::ShowMillis => "ShowMillis",
            ContentKey::ActKeyboard => "ActKeyboard",
        };
        write!(f, "{}", str_)
    }
}

pub struct MainContents {
    pub contents: IndexMap<ContentKey, Box<dyn SettingsItem>>,
    main_style: Style,
    highl_style: Style,
    border_style: Style,
}

impl MainContents {
    pub fn new() -> Self {
        let tick_rate = ContentKey::TickRate.to_content_item();
        let time_show_millis = ContentKey::ShowMillis.to_content_item();
        let active_kbd = ContentKey::ActKeyboard.to_content_item();
        let mut this = Self {
            contents: IndexMap::new(),
            main_style: Style::default().bg(ui::BACKGROUND),
            highl_style: Style::default().fg(ui::BACKGROUND).bg(ui::BORDER),
            border_style: Style::default().fg(ui::BORDER).bg(ui::BACKGROUND),
        };
        this.contents.insert(ContentKey::TickRate, tick_rate);
        this.contents
            .insert(ContentKey::ShowMillis, time_show_millis);
        this.contents.insert(ContentKey::ActKeyboard, active_kbd);
        this
    }

    pub fn get_setting(&self, key: ContentKey) -> Option<&Box<dyn SettingsItem>> {
        self.contents.get(&key)
    }

    pub fn get_mut_setting(&mut self, key: ContentKey) -> Option<&mut Box<dyn SettingsItem>> {
        self.contents.get_mut(&key)
    }

    pub fn set_setting(&mut self, key: ContentKey, content: Box<dyn SettingsItem>) {
        self.contents.insert(key, content);
    }

    pub fn draw_item(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        key: &ContentKey,
        area: Rect,
    ) -> Result<(), AppError> {
        let clear = Paragraph::new("").style(self.main_style);
        f.render_widget(clear, area);
        self.contents
            .get(key)
            .ok_or(AppError::SettingNotFound)?
            .draw(
                f,
                area,
                self.main_style,
                self.highl_style,
                self.border_style,
            )
    }

    pub fn get_active_list<'a>(&'a self) -> Vec<ListItem> {
        self.contents
            .iter()
            .map(|(_, item)| item.to_listitem())
            .collect()
    }

    pub fn get_active_table(&self) -> Table {
        let rows = self
            .contents
            .iter()
            .map(|(value, key)| Row::new([value.to_string(), key.to_string()]));
        return Table::new(rows);
    }
}

#[derive(Default)]
pub struct ContentState<T> {
    list_state: ListState,
    entry_state: EntryState,
    phantomdata: PhantomData<T>,
}

impl ContentState<bool> {
    fn to_list_state(&self) -> ListState {
        self.list_state.clone()
    }
}

impl ContentState<String> {
    fn to_list_state(&self) -> ListState {
        self.list_state.clone()
    }
}

impl ContentState<u32> {
    fn to_entry_state(&self) -> EntryState {
        self.entry_state.clone()
    }
}

pub struct ContentItem<T: ContentItemType> {
    key: ContentKey,
    state: Option<T>,
    options: Vec<T>,
    widget_state: ContentState<T>,
}

impl ContentItem<u32> {
    pub fn new(key: ContentKey, value: u32, range: (u32, u32)) -> Self {
        return Self {
            key,
            state: Some(value),
            options: vec![range.0, range.1],
            widget_state: ContentState::default(),
        };
    }
}

impl ContentItem<bool> {
    pub fn new(key: ContentKey, state: bool) -> Self {
        return Self {
            key,
            state: Some(state),
            options: vec![],
            widget_state: ContentState::default(),
        };
    }
}

impl ContentItem<String> {
    pub fn new(key: ContentKey, options: Vec<String>) -> Self {
        return Self {
            key,
            state: options.get(0).cloned(),
            options,
            widget_state: ContentState::default(),
        };
    }
}

impl SettingsItem for ContentItem<bool> {
    fn get_key(&self) -> ContentKey {
        return self.key.clone();
    }

    fn to_bool(&self) -> Result<bool, AppError> {
        return Ok(self.state.unwrap());
    }

    fn to_value(&self) -> Result<u32, AppError> {
        return Ok(match self.state {
            Some(true) => 1,
            _ => 0,
        });
    }

    fn handle_event(&mut self, event: Event) -> WindowEvent {
        match event.into() {
            Key::Char('j') => self.widget_state.list_state.select(Some(
                self.widget_state
                    .list_state
                    .selected()
                    .map_or(0, |idx| (idx + 1) % 2),
            )),
            Key::Char('k') => self.widget_state.list_state.select(Some(
                self.widget_state
                    .list_state
                    .selected()
                    .map_or(1, |idx| (idx + 1) % 2),
            )),
            Key::Enter => match self.widget_state.list_state.selected() {
                Some(0) => self.state = Some(true),
                Some(1) => self.state = Some(false),
                _ => {}
            },
            Key::Esc => return WindowEvent::ExitWindow,
            _ => {}
        }
        return WindowEvent::NoEvent;
    }

    fn draw(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        style: Style,
        highl_style: Style,
        border_style: Style,
    ) -> Result<(), AppError> {
        let split = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);
        let block = Block::default()
            .style(style)
            .border_style(border_style)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let selection = Paragraph::new(format!("Current: {}", self.state.unwrap_or(false)))
            .block(block.clone());
        f.render_widget(selection, split[0]);

        let widget = self.to_list()?.highlight_style(highl_style).block(block);
        f.render_stateful_widget(widget, split[1], &self.widget_state.to_list_state());
        Ok(())
    }
}

impl SettingsItem for ContentItem<String> {
    fn get_key(&self) -> ContentKey {
        return self.key.clone();
    }
    fn to_bool(&self) -> Result<bool, AppError> {
        return Err(AppError::SettingsType(format!(
            "{} is not a boolean setting",
            self.key
        )));
    }

    fn to_value(&self) -> Result<u32, AppError> {
        return Err(AppError::SettingsType(format!(
            "{} is not a numeric setting",
            self.key
        )));
    }

    fn handle_event(&mut self, event: Event) -> WindowEvent {
        match event.into() {
            Key::Char('j') => self.widget_state.list_state.select(Some(
                self.widget_state
                    .list_state
                    .selected()
                    .map_or(0, |idx| (idx + 1) % self.options.len()),
            )),
            Key::Char('k') => self.widget_state.list_state.select(Some(
                self.widget_state
                    .list_state
                    .selected()
                    .map_or(self.options.len(), |idx| {
                        (idx + self.options.len() - 1) % self.options.len()
                    }),
            )),
            Key::Enter => {
                self.state = self
                    .options
                    .get(self.widget_state.list_state.selected().unwrap_or(0))
                    .cloned()
            }
            Key::Esc => return WindowEvent::ExitWindow,
            _ => {}
        }
        WindowEvent::NoEvent
    }

    fn draw(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        style: Style,
        highl_style: Style,
        border_style: Style,
    ) -> Result<(), AppError> {
        let split = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);
        let block = Block::default()
            .style(style)
            .border_style(border_style)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let selection = Paragraph::new(format!(
            "Current: {}",
            self.state.clone().unwrap_or("".to_string())
        ))
        .block(block.clone());
        f.render_widget(selection, split[0]);

        let widget = self.to_list()?.highlight_style(highl_style).block(block);
        f.render_stateful_widget(widget, split[1], &self.widget_state.to_list_state());
        Ok(())
    }
}

impl SettingsItem for ContentItem<u32> {
    fn get_key(&self) -> ContentKey {
        return self.key.clone();
    }
    fn to_bool(&self) -> Result<bool, AppError> {
        return Err(AppError::SettingsType(format!(
            "{} is not a boolean setting",
            self.key
        )));
    }
    fn to_value(&self) -> Result<u32, AppError> {
        return Ok(self.state.unwrap());
    }

    fn handle_event(&mut self, event: Event) -> WindowEvent {
        match event.into() {
            Key::Char(char_) if char_.is_digit(10) => self.widget_state.entry_state.push(char_),
            Key::Backspace => self.widget_state.entry_state.pop(),
            Key::Enter => {
                self.state = match self
                    .widget_state
                    .entry_state
                    .get_active_field()
                    .parse::<u32>()
                {
                    Ok(num) if num >= self.options[0] && num <= self.options[1] => Some(num),
                    _ => self.state,
                }
            }
            Key::Esc => return WindowEvent::ExitWindow,
            _ => {}
        };
        WindowEvent::NoEvent
    }

    fn draw(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        style: Style,
        _highl_style: Style,
        border_style: Style,
    ) -> Result<(), AppError> {
        let split = Layout::default()
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);
        let block = Block::default()
            .style(style)
            .border_style(border_style)
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        let selection = Paragraph::new(format!(
            "Current: {}  ({}-{})",
            self.state.clone().unwrap_or(self.options[0]),
            self.options[0],
            self.options[1]
        ))
        .block(block.clone());
        f.render_widget(selection, split[0]);
        let widget = self.to_entry()?.block(block);
        f.render_stateful_widget(widget, split[1], &self.widget_state.to_entry_state());

        Ok(())
    }
}

impl<T: ContentItemType> ToListItem for ContentItem<T> {
    fn to_listitem(&self) -> ListItem {
        return ListItem::new(self.to_string());
    }
}

impl ToList for ContentItem<bool> {
    fn to_list(&self) -> Result<List, AppError> {
        return Ok(List::new([ListItem::new("true"), ListItem::new("false")]));
    }
}

impl ToList for ContentItem<String> {
    fn to_list(&self) -> Result<List, AppError> {
        let items: Vec<ListItem> = self
            .options
            .iter()
            .map(|item| ListItem::new(item.to_string()))
            .collect();
        return Ok(List::new(items));
    }
}

impl ToList for ContentItem<u32> {
    fn to_list(&self) -> Result<List, AppError> {
        return Err(AppError::SettingsType(format!(
            "{} can not be displayed as a list",
            self.key
        )));
    }
}

impl ToEntry for ContentItem<bool> {
    fn to_entry(&self) -> Result<Entry, AppError> {
        Err(AppError::SettingsType(
            "{} can not be displayed as an entry".to_string(),
        ))
    }
}

impl ToEntry for ContentItem<String> {
    fn to_entry(&self) -> Result<Entry, AppError> {
        Err(AppError::SettingsType(
            "{} can not be displayed as an entry".to_string(),
        ))
    }
}

impl ToEntry for ContentItem<u32> {
    fn to_entry(&self) -> Result<Entry, AppError> {
        let field_width = self
            .options
            .get(1)
            .ok_or(AppError::ImpossibleState("".to_string()))?
            .to_string()
            .len()
            + 1;
        return Ok(Entry::default().field_width(field_width as u16));
    }
}

impl<T: ContentItemType> ToString for ContentItem<T> {
    fn to_string(&self) -> String {
        return self.state.clone().unwrap_or_default().to_string();
    }
}
