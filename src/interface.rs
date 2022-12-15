#![allow(private_in_public)]
use core::str::Chars;
use std::fmt::Display;
use std::collections::HashMap;
use std::rc::Rc;

const TEST_CHAR: char = '*';

pub trait Widget {
    fn draw(&self, _: &mut InterFace);
    fn set_position(&mut self, _: Position);
    fn set_size(&mut self, _: WidgetSize);
    fn get_size(&self) -> &WidgetSize;
}

pub trait Container {
    fn child(&mut self, widget: Box<dyn Widget>);
    fn fit_child(&mut self, state: bool);
    fn get_child(&self) -> &mut dyn Widget;
    fn has_child(&self) -> bool;
}

trait Bordered {
    fn set_border(&mut self, border: Border);
}

pub struct InterFace {
    width:  usize,
    height: usize,
    window: Vec<char>,
}

impl InterFace {
    fn new(height: usize, width: usize) -> Self {
        return Self { width, height, window: vec!(TEST_CHAR; height * width) }
    }
    fn insert_chars(&mut self, chars: &mut Chars, range: Vec<usize>) {
        for index in range.into_iter() {
            self.window[index] = chars.next().unwrap_or('\0');
        }
    }
    fn insert_char(&mut self, position: (usize, usize), charr: char) {
        self.window[position.0 + (position.1 * self.width)] = charr
    }
    fn draw_border(&mut self, size: &WidgetSize, position: &Position) {
        let lower_right = Position { x: size.width + position.x - 1, y: size.height + position.y - 1 };
        let horz_range  = (position.x..lower_right.x).collect::<Vec<usize>>();
        let vert_range  = (position.y..lower_right.y).collect::<Vec<usize>>();

        self.fill_line(Direction::Vertical  , position.x, &vert_range   , '│');
        self.fill_line(Direction::Vertical  , lower_right.x, &vert_range, '│');
        self.fill_line(Direction::Horizontal, position.y, &horz_range   , '─');
        self.fill_line(Direction::Horizontal, lower_right.y, &horz_range, '─');

        self.insert_char(position.to_tuple()        , '┌');
        self.insert_char((position.x, lower_right.y), '└');
        self.insert_char((lower_right.x, position.y), '┐');
        self.insert_char(lower_right.to_tuple()     , '┘');
    }
    fn fill_line(&mut self, direction: Direction, line_nr: usize, range: &Vec<usize>, charr: char) {
        match direction {
            Direction::Horizontal => {
                for index in range.clone() {
                    self.window[index + line_nr * self.width] = charr;
                }
            }
            Direction::Vertical => {
                for index in range {
                    self.window[index * self.width + line_nr] = charr;
                }
            }
        }
    }
}

impl Display for InterFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut string = String::new();
        for line in self.window.chunks(self.width) {
            string.push_str(&line.into_iter().map(|charr| if charr == &'\0' {return &' '} else {return charr}).collect::<String>());
            string.push('\n')
        }
        write!(f, "{}", string)
    }
}

enum Direction {
    Horizontal,
    Vertical,
}

enum Border {
    Full,
    Dots,
    Striped,
    None,
}

#[derive(Debug)]
struct Position {
    x: usize,
    y: usize,
}

impl Default for Position {
    fn default() -> Self {
        return Self { x: 0, y: 0 }
    }
}

impl Position {
    fn offset_from(position: &Position, offset: usize) -> Self {
        return Self { x: position.x + offset, y: position.y + offset }
    }
    fn get_flat_index(&self, width: usize) -> usize {
        return width * self.y + self.x
    }
    fn to_tuple(&self) -> (usize, usize) {
        return (self.x, self.y)
    }
}

#[derive(Debug)]
struct WidgetSize {
    width:  usize,
    height: usize,
}

impl WidgetSize {
    fn wrap(size: &WidgetSize) -> Self {
        return Self { width: size.width + 2, height: size.height + 2 }
    }
    fn inside(size: &WidgetSize) -> Self {
        return Self { width: size.width - 2, height: size.height - 2 }
    }
}

pub struct WidgetRelation {
    parent: &'static dyn Widget,
    child:  Option<&'static dyn Widget>,
}

pub struct Window {
    width:     usize,
    height:    usize,
    border:    Border,
    relation:  Rc<ProgramRelations>,
    interface: InterFace,
}

impl Window {
    pub fn new(width: usize, height: usize, relation: Rc<ProgramRelations>) -> Box<Self> {
        return Box::new( Self { height, width, border: Border::None, relation, interface: InterFace::new(height, width) } )
    }
    fn draw(&mut self) -> String {
        if self.has_child() {
            self.get_child().draw(&mut self.interface);
        }
        return self.interface.to_string();
    }
    pub fn present(&mut self) {
        println!("{}", self.draw());
    }
}

impl Container for Window {
    fn child(&mut self, widget: Box<dyn Widget>) {
        todo!()
    }
    fn fit_child(&mut self, _state: bool) {}

    fn get_child(&self) -> &mut dyn Widget {
        todo!()
    }
    fn has_child(&self) -> bool { todo!() }
}

impl Bordered for Window {
    fn set_border(&mut self, border: Border) { self.border = border }
}

pub struct Label {
    parent_id: u32,
    relation:  Rc<ProgramRelations>,
    text:      String,
    size:      WidgetSize,
    position:  Position,
    wrapping:  bool,
}

impl Label {
    pub fn new(text: &str, relation: Rc<ProgramRelations>) -> Box<Self> {
        return Box::new( Self { parent_id: 0, relation, text: text.to_string(), size: WidgetSize { width: text.len(), height: 1 }, position: Position::default(), wrapping: true } )
    }
}

impl Widget for Label {
    fn draw(&self, interface: &mut InterFace) {
        let range = get_sized_range(&self.position, &self.size, interface.width);
        println!("{:?}, {:?}, {:?}", range, self.position, self.size);
        interface.insert_chars(&mut self.text.chars(), range)
    }
    fn set_position(&mut self, position: Position) { self.position = position }
    fn get_size(&self) -> &WidgetSize { &self.size }
    fn set_size(&mut self, size: WidgetSize) { self.size = size }
}

pub struct Frame {
    parent_id: u32,
    relation:  Rc<ProgramRelations>,
    size:      WidgetSize,
    position:  Position,
    border:    Border,
    fit_child: bool,
}

impl Frame {
    pub fn new(width: usize, height: usize, relation: Rc<ProgramRelations>) -> Box<Self> {
        return Box::new(Self { parent_id: 0, relation, size: WidgetSize { width, height }, position: Position::default(), border: Border::Full, fit_child: true })
    }
}

impl Widget for Frame {
    fn draw(&self, interface: &mut InterFace) {
        if self.has_child() {
            self.get_child().draw(interface)
        }
        interface.draw_border(&self.size, &self.position)
    }
    fn set_position(&mut self, position: Position) { 
        self.position = position;
        if self.has_child() {
            self.get_child().set_position(Position::offset_from(&self.position, 1))
        }
    }
    fn get_size(&self) -> &WidgetSize { &self.size }
    fn set_size(&mut self, size: WidgetSize) { self.size = size }
}

impl Container for Frame {
    fn child(&mut self, mut widget: Box<dyn Widget>) {
        widget.set_position(Position::offset_from(&self.position, 1));
        if self.fit_child {
            self.set_size(WidgetSize::wrap(widget.get_size()));
        } else {
            widget.set_size(WidgetSize::inside(&self.size));
        }
    }
    fn fit_child(&mut self, state: bool) {
        assert!(!self.has_child(), "Cannot change Frame parameters after Child is attached");
        self.fit_child = state;
    }
    fn get_child(&self) -> &mut dyn Widget { todo!() }
    fn has_child(&self) -> bool { todo!() }
}

impl Bordered for Frame {
    fn set_border(&mut self, border: Border) { self.border = border }
}

fn get_sized_range(position: &Position, size: &WidgetSize, interface_width: usize) -> Vec<usize> {
    let mut range = vec!();
    for line in (0 + position.y)..(size.height + position.y) {
        let left  = interface_width * line + position.x;
        let right = interface_width * line + size.width + position.x;
        range.append(&mut (left..right).collect::<Vec<usize>>())
    }
    return range
}

pub struct ProgramRelations {
    widgets: HashMap<u16, Box<dyn Widget>>,
}

impl ProgramRelations {
    pub fn new() -> Self {
        return Self { widgets: HashMap::new() }
    }
    fn add_widget(&mut self, widget: Box<dyn Widget>) -> u16 {
        let id = self.widgets.keys().max().unwrap() + 1;
        self.widgets.insert(id, widget);
        return id
    }
    fn get_by_id(&self) {
        todo!()
    }
}
