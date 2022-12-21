use tui::text::Text;
use tui::widgets::Block;
use crossterm::event::KeyCode;
use tui::{style::Style, widgets::Widget};
use tui::layout::Rect;

pub struct Dialog<'a> {
    block:       Option<Block<'a>>,
    title:       String,
    style:       Style,
    confirm_key: Option<KeyCode>,
    cancel_key:  Option<KeyCode>,
}

impl<'a> Dialog<'a> {
    pub fn default() -> Self {
        return Dialog {
            block:       Some(Block::default()),
            title:       "".to_string(),
            style:       Style::default(),
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

impl<'a> Widget for Dialog<'a> {
    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer) {
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

        // display the usable keys on the bottom if space allows it and keys are initialized
        let key_info = format!("<{:?}>Cancel  <{:?}>Confirm", self.cancel_key.unwrap(), self.confirm_key.unwrap());
        if widget_area.height >= 4 && widget_area.width > key_info.len() as u16 && self.cancel_key.is_some() && self.confirm_key.is_some() {
            let key_line = Text::raw(&key_info);
            buf.set_spans(widget_area.x + widget_area.width - 1 - key_info.len() as u16, widget_area.y + widget_area.height - 1, &key_line.lines[0], widget_area.width);
        }
    }
}
