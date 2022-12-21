use tui::text::Text;
use tui::widgets::Block;
use crossterm::event::KeyCode;
use tui::{style::Style, widgets::{Widget, StatefulWidget}};
use tui::layout::Rect;

pub struct EntryState {
    field: String,
    cursor_pos: Option<(u16, u16)>,
}

impl EntryState {
    pub fn default() -> Self {
        return EntryState { field: "".to_string(), cursor_pos: Some((0, 0)) }
    }
    pub fn push(&mut self, charr: char) {
        self.field.push(charr);
    }
    pub fn get_field(&self) -> String {
        return self.field.clone()
    }

    pub fn pop(&mut self) {
        self.field.pop();
    }

    pub fn show_cursor(mut self) -> Self {
        self.cursor_pos = Some((0, 0));
        self
    }

    pub fn hide_cursor(mut self) -> Self {
        self.cursor_pos = Some((0, 0));
        self
    }

    pub fn get_cursor(&self) -> Option<(u16, u16)> {
        return self.cursor_pos
    }
}

pub struct Entry<'a> {
    block:       Option<Block<'a>>,
    title:       String,
    field_width: u16,
    style:       Style,
    field_style: Style,
    confirm_key: Option<KeyCode>,
    cancel_key:  Option<KeyCode>,
}

impl<'a> Entry<'a> {
    pub fn default() -> Self {
        return Entry {
            block:       Some(Block::default()),
            title:       "".to_string(),
            field_width: 10,
            style:       Style::default(),
            field_style: Style::default(),
            confirm_key: None,
            cancel_key:  None,
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

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn keys(mut self, cancel_key: KeyCode, confirm_key: KeyCode) -> Self {
        self.confirm_key = Some(confirm_key);
        self.cancel_key  = Some(cancel_key);
        self
    }
}

impl<'a> StatefulWidget for Entry<'a> {
    type State = EntryState;

    fn render(mut self, area: Rect, buf: &mut tui::buffer::Buffer, state: &mut Self::State) {
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

        // always keep the entry area two characters bigger than the entered frase
        // but only increase after it has exceeded the requested start length
        if state.field.len() + 2 > self.field_width as usize {
            self.field_width = state.field.len() as u16 + 2
        }

        // calculate the area of the entry bar
        let mut entry_area = Rect::default();
        if widget_area.width > self.field_width && widget_area.height > 3 {
            entry_area = Rect{ 
                x:      (widget_area.width - self.field_width) / 2 + widget_area.x,
                y:      widget_area.height / 2 + widget_area.y,
                width:  self.field_width,
                height: 1
            };
        }

        buf.set_style(entry_area, self.field_style);
        /* print all empty characters for the entire area of the entry widget to make it override
        any other widget below */
        let widget_empty = Text::raw(" ".repeat(widget_area.width as usize));
        for i in 0..widget_area.height {
            buf.set_spans(widget_area.x, widget_area.y + i as u16, &widget_empty.lines[0], widget_area.width);
        }

        // showing title two line above the entry bar
        if widget_area.width > self.title.len() as u16 {
            let title = Text::raw(self.title);
            for line in title.lines {
                buf.set_spans(
                    (widget_area.width - line.width() as u16) / 2 + widget_area.x,
                    widget_area.height / 2 + widget_area.y - 2, 
                    &line, 
                    widget_area.width
                );
            }
        }
        // create a span to show the entered information padded by underscores
        let mut padded_field = state.get_field();
        if self.field_width > state.field.len() as u16 {
            padded_field.push_str(&"_".repeat(self.field_width as usize - state.field.len()));
        }
        let line = Text::raw(&padded_field);
        for line in line.lines {
            buf.set_spans(entry_area.x, entry_area.y, &line, widget_area.width);
        }

        // setting cursor just after last character
        if state.get_cursor().is_some() {
            state.cursor_pos = Some((entry_area.x + state.field.len() as u16, entry_area.y));
        }

        // display the usable keys on the bottom if space allows it and keys are initialized
        let key_info = format!("<{:?}>Cancel  <{:?}>Confirm", self.cancel_key.unwrap(), self.confirm_key.unwrap());
        if widget_area.height >= 4 && widget_area.width > key_info.len() as u16 && self.cancel_key.is_some() && self.confirm_key.is_some() {
            let key_line = Text::raw(&key_info);
            buf.set_spans(widget_area.x + widget_area.width - 1 - key_info.len() as u16, widget_area.y + widget_area.height - 1, &key_line.lines[0], widget_area.width);
        }
    }
}

impl<'a> Widget for Entry<'a> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let mut state = EntryState::default();
        StatefulWidget::render(self, area, buf, &mut state)
    }
}
