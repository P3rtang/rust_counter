use std::cell::RefCell;
use crossterm::event::KeyCode;
use tui::{layout::Rect, text::Text, style::Style, widgets::{WidgetState, StatefulWidget, Widget, Block}};

#[derive(Clone)]
pub struct EntryState {
    fields: Vec<String>,
    active_field: usize,
    cursor_pos: RefCell<Option<(u16, u16)>>,
}

impl EntryState {
    pub fn new(size: usize) -> Self {
        Self { fields: vec![String::new(); size], active_field: 0, cursor_pos: None.into() }
    }

    pub fn push(&mut self, charr: char) {
        self.fields[self.active_field].push(charr);
    }

    pub fn push_str(&mut self, string: impl Into<String>) {
        self.fields[self.active_field].push_str(&string.into())
    }

    pub fn set_field(&mut self, field: impl Into<String>) {
        self.fields[self.active_field] = field.into()
    }

    pub fn get_field(&self, idx: usize) -> String {
        self.fields[idx].clone()
    }

    pub fn get_active_field(&self) -> &String {
        &self.fields[self.active_field]
    }
    pub fn set_active_field(&mut self, idx: usize) {
        self.active_field = idx
    }

    pub fn get_fields(&self) -> Vec<String> {
        self.fields.clone()
    }

    pub fn next(&mut self) {
        self.active_field += 1;
        self.active_field %= self.fields.len();
    }
    pub fn prev(&mut self) {
        // avoid underflow
        self.active_field += self.fields.len() - 1;
        self.active_field %= self.fields.len();
    }

    pub fn new_field(&mut self, default_value: impl Into<String>) {
        self.fields.push(default_value.into());
        self.active_field = self.fields.len() - 1;
    }

    pub fn pop(&mut self) {
        self.fields[self.active_field].pop();
    }

    pub fn show_cursor(mut self) -> Self {
        self.cursor_pos = Some((0, 0)).into();
        self
    }

    pub fn hide_cursor(mut self) -> Self {
        self.cursor_pos = None.into();
        self
    }

    pub fn get_cursor(&self) -> Option<(u16, u16)> {
        self.cursor_pos.borrow().clone()
    }
}

impl WidgetState for EntryState {}

impl Default for EntryState {
    fn default() -> Self {
        EntryState {
            fields: vec![String::new(); 1],
            active_field: 0,
            cursor_pos: RefCell::new(Some((0, 0))),
        }
    }
}

pub struct Entry<'a> {
    block: Option<Block<'a>>,
    message: String,
    field_width: u16,
    style: Style,
    field_style: Style,
    confirm_key: Option<KeyCode>,
    cancel_key: Option<KeyCode>,
}

impl<'a> Entry<'a> {
    pub fn default() -> Self {
        Entry {
            block: Some(Block::default()),
            message: "".to_string(),
            field_width: 10,
            style: Style::default(),
            field_style: Style::default(),
            confirm_key: None,
            cancel_key: None,
        }
    }
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn field_width(mut self, width: u16) -> Self {
        self.field_width = width;
        self
    }

    pub fn field_style(mut self, style: Style) -> Self {
        self.field_style = style;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.message = title.into();
        self
    }

    pub fn keys(mut self, cancel_key: KeyCode, confirm_key: KeyCode) -> Self {
        self.confirm_key = Some(confirm_key);
        self.cancel_key = Some(cancel_key);
        self
    }
}

impl<'a> StatefulWidget for Entry<'a> {
    type State = EntryState;

    fn render(mut self, area: Rect, buf: &mut tui::buffer::Buffer, state: &Self::State) {
        // get the area of the widget itself (this is to exclude the border from the area)
        buf.set_style(area, self.style);
        let widget_area = match self.block {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        // calculate the area of the entry bar
        let mut entry_area = widget_area;
        if widget_area.width > self.field_width && widget_area.height > 3 {
            entry_area = Rect {
                x: (widget_area.width - self.field_width) / 2 + widget_area.x,
                y: widget_area.height / 2 + widget_area.y,
                width: self.field_width,
                height: 1,
            };
        }

        buf.set_style(entry_area, self.field_style);

        let message = Text::raw(self.message);
        for (line_nr, line) in message.lines.iter().enumerate() {
            if widget_area.width < line.width() as u16 {
                buf.set_spans(
                    widget_area.x,
                    widget_area.y + line_nr as u16,
                    line,
                    widget_area.width,
                );
            } else if widget_area.height <= line_nr as u16 {
                continue;
            } else {
                buf.set_spans(
                    (widget_area.width - line.width() as u16) / 2 + widget_area.x,
                    widget_area.height / 2 + widget_area.y - 2 + line_nr as u16,
                    line,
                    widget_area.width,
                );
            }
        }
        // create a span to show the entered information padded by underscores
        for (field_nr, field) in state.get_fields().iter().enumerate() {
            // always keep the entry area two characters bigger than the entered frase
            // but only increase after it has exceeded the requested start length
            if field.len() + 2 > self.field_width as usize {
                self.field_width = field.len() as u16 + 2
            }

            let mut padded_field = field.clone();
            if self.field_width > field.len() as u16 {
                padded_field.push_str(&"_".repeat(self.field_width as usize - field.len()));
            }
            let line = Text::raw(&padded_field);
            buf.set_spans(
                entry_area.x,
                entry_area.y + field_nr as u16,
                &line.lines[0],
                widget_area.width,
            );
        }

        // setting cursor just after last character
        if state.get_cursor().is_some() {
            state.cursor_pos.swap(&RefCell::new(Some((
                entry_area.x + state.get_active_field().len() as u16,
                entry_area.y,
            ))));
        }

        // display the usable keys on the bottom if space allows it and keys are initialized
        let key_info = if self.confirm_key.is_some() && self.cancel_key.is_some() {
            format!(
                "<{:?}>Cancel  <{:?}>Confirm",
                self.cancel_key.unwrap(),
                self.confirm_key.unwrap()
            )
        } else {"".to_string()};

        if widget_area.height >= 4
            && widget_area.width > key_info.len() as u16
            && self.cancel_key.is_some()
            && self.confirm_key.is_some()
        {
            let key_line = Text::raw(&key_info);
            buf.set_spans(
                widget_area.x + widget_area.width - 1 - key_info.len() as u16,
                widget_area.y + widget_area.height - 1,
                &key_line.lines[0],
                widget_area.width,
            );
        }
    }
}

impl<'a> Widget for Entry<'a> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let state = EntryState::default();
        StatefulWidget::render(self, area, buf, &state)
    }
}
