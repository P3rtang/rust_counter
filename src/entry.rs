use tui::style::Style;
use tui::widgets::{Widget, StatefulWidget};
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
}

impl<'a> Entry<'a> {
    pub fn default() -> Self {
        return Entry {
            block:       Some(Block::default()),
            title:       "".to_string(),
            field_width: 10,
            style:       Style::default(),
            field_style: Style::default(),
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
}

impl<'a> StatefulWidget for Entry<'a> {
    type State = EntryState;

    fn render(mut self, area: Rect, buf: &mut tui::buffer::Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let widget_area = match self.block {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if state.field.len() + 2 > self.field_width as usize {
            self.field_width = state.field.len() as u16 + 2
        }

        let mut entry_area = Rect::default();
        if widget_area.width > self.field_width && widget_area.height > 3 {
            entry_area = Rect{ x: (widget_area.width - self.field_width) / 2 + widget_area.x, y: widget_area.height / 2 + widget_area.y, width: self.field_width, height: 1 };
        }

        buf.set_style(entry_area, self.field_style);

        let widget_empty = Text::raw(" ".repeat(widget_area.width as usize));
        for i in 0..widget_area.height {
            buf.set_spans(widget_area.x, widget_area.y + i as u16, &widget_empty.lines[0], widget_area.width);
        }

        let title = Text::raw(self.title);
        for line in title.lines {
            buf.set_spans((widget_area.width - line.width() as u16) / 2 + widget_area.x, widget_area.height / 2 + widget_area.y - 2, &line, widget_area.width);
        }

        let mut padded_field = state.get_field();
        padded_field.push_str(&"_".repeat(self.field_width as usize - state.field.len()));
        let line = Text::raw(&padded_field);
        for line in line.lines {
            buf.set_spans(entry_area.x, entry_area.y, &line, widget_area.width);
        }

        if state.get_cursor().is_some() {
            state.cursor_pos = Some((entry_area.x + state.field.len() as u16, entry_area.y));
        }
    }
}

impl<'a> Widget for Entry<'a> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let mut state = EntryState::default();
        StatefulWidget::render(self, area, buf, &mut state)
    }
}

use tui::{buffer::Buffer, layout::Corner, widgets::Block, text::Text};

#[derive(Debug, Clone, Default)]
pub struct ListState {
    offset: usize,
    selected: Option<usize>,
}

impl ListState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem<'a> {
    content: Text<'a>,
    style: Style,
}

impl<'a> ListItem<'a> {
    pub fn new<T>(content: T) -> ListItem<'a>
    where
        T: Into<Text<'a>>,
    {
        ListItem {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> ListItem<'a> {
        self.style = style;
        self
    }

    pub fn height(&self) -> usize {
        self.content.height()
    }
}

/// A widget to display several items among which one can be selected (optional)
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders, List, ListItem};
/// # use tui::style::{Style, Color, Modifier};
/// let items = [ListItem::new("Item 1"), ListItem::new("Item 2"), ListItem::new("Item 3")];
/// List::new(items)
///     .block(Block::default().title("List").borders(Borders::ALL))
///     .style(Style::default().fg(Color::White))
///     .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
///     .highlight_symbol(">>");
/// ```
#[derive(Debug, Clone)]
pub struct List<'a> {
    block: Option<Block<'a>>,
    items: Vec<ListItem<'a>>,
    /// Style used as a base style for the widget
    style: Style,
    start_corner: Corner,
    /// Style used to render selected item
    highlight_style: Style,
    /// Symbol in front of the selected item (Shift all items to the right)
    highlight_symbol: Option<&'a str>,
    /// Whether to repeat the highlight symbol for each line of the selected item
    repeat_highlight_symbol: bool,
}

impl<'a> List<'a> {
    pub fn new<T>(items: T) -> List<'a>
    where
        T: Into<Vec<ListItem<'a>>>,
    {
        List {
            block: None,
            style: Style::default(),
            items: items.into(),
            start_corner: Corner::TopLeft,
            highlight_style: Style::default(),
            highlight_symbol: None,
            repeat_highlight_symbol: false,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> List<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> List<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> List<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> List<'a> {
        self.highlight_style = style;
        self
    }

    pub fn repeat_highlight_symbol(mut self, repeat: bool) -> List<'a> {
        self.repeat_highlight_symbol = repeat;
        self
    }

    pub fn start_corner(mut self, corner: Corner) -> List<'a> {
        self.start_corner = corner;
        self
    }

    fn get_items_bounds(
        &self,
        selected: Option<usize>,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(self.items.len().saturating_sub(1));
        let mut start = offset;
        let mut end = offset;
        let mut height = 0;
        for item in self.items.iter().skip(offset) {
            if height + item.height() > max_height {
                break;
            }
            height += item.height();
            end += 1;
        }

        let selected = selected.unwrap_or(0).min(self.items.len() - 1);
        while selected >= end {
            height = height.saturating_add(self.items[end].height());
            end += 1;
            while height > max_height {
                height = height.saturating_sub(self.items[start].height());
                start += 1;
            }
        }
        while selected < start {
            start -= 1;
            height = height.saturating_add(self.items[start].height());
            while height > max_height {
                end -= 1;
                height = height.saturating_sub(self.items[end].height());
            }
        }
        (start, end)
    }
}

impl<'a> StatefulWidget for List<'a> {
    type State = ListState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        if self.items.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;

        let (start, end) = self.get_items_bounds(state.selected, state.offset, list_height);
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let blank_symbol = " ".repeat(highlight_symbol.len());

        let mut current_height = 0;
        let has_selection = state.selected.is_some();
        for (i, item) in self
            .items
            .iter_mut()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
        {
            let (x, y) = match self.start_corner {
                Corner::BottomLeft => {
                    current_height += item.height() as u16;
                    (list_area.left(), list_area.bottom() - current_height)
                }
                _ => {
                    let pos = (list_area.left(), list_area.top() + current_height);
                    current_height += item.height() as u16;
                    pos
                }
            };
            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: item.height() as u16,
            };
            let item_style = self.style.patch(item.style);
            buf.set_style(area, item_style);

            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);
            for (j, line) in item.content.lines.iter().enumerate() {
                // if the item is selected, we need to display the hightlight symbol:
                // - either for the first line of the item only,
                // - or for each line of the item if the appropriate option is set
                let symbol = if is_selected && (j == 0 || self.repeat_highlight_symbol) {
                    highlight_symbol
                } else {
                    &blank_symbol
                };
                let (elem_x, max_element_width) = if has_selection {
                    let (elem_x, _) = buf.set_stringn(
                        x,
                        y + j as u16,
                        symbol,
                        list_area.width as usize,
                        item_style,
                    );
                    (elem_x, (list_area.width - (elem_x - x)) as u16)
                } else {
                    (x, list_area.width)
                };
                buf.set_spans(elem_x, y + j as u16, line, max_element_width as u16);
            }
            if is_selected {
                buf.set_style(area, self.highlight_style);
            }
        }
    }
}

impl<'a> Widget for List<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

