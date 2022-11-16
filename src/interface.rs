use std::cmp::max;
use std::io::{stdout, Write};
use std::fmt::Display;

const EMPTY_CHAR: &str = " ";

pub trait InterFaceChild {
    fn draw(&mut self);
    fn size(&self) -> (u16, u16);
    fn build(&mut self, size: (u16, u16), position: (u16, u16)) -> &InterFace;
}

pub trait InterFaceParent<I> where I: InterFaceChild {
    fn attach(&mut self, widget: I) -> Result<(), ()> ;
    fn build(&mut self); 
}

pub enum Border {
    None,
    Full,
}

pub struct InterFace {
    size: (u16, u16),
    position: (u16, u16),
    interface: Vec<Vec<char>>
}

impl InterFace {
    fn new(position: (u16, u16), size: (u16, u16)) -> Self {
        let interface_line = vec!('\0'; size.0 as usize);
        let interface = vec!(interface_line; size.1 as usize);
        return InterFace { size, position, interface }
    }
    fn insert(&mut self, position: (u16, u16), interface: &InterFace) -> Result<(), ()> {
        let size = interface.size;
        for y_index in 0..size.1 {
            for x_index in 0..size.0 {
                self.interface[(y_index + position.1) as usize][(x_index + position.0) as usize] = interface.interface[y_index as usize][x_index as usize]
            }
        }
        Ok(())
    }
}

impl Display for InterFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut write_vec: Vec<String> = vec!();
        for line in &self.interface {
            let mut line_string = line.into_iter().collect::<String>();
            line_string = line_string.replace('\0', EMPTY_CHAR);
            line_string.push('\n');
            write_vec.push(line_string);
        }
        write!(f, "{}", write_vec.into_iter().collect::<String>())
    }
}

impl Default for InterFace {
    fn default() -> Self {
        let interface = vec!(vec!('\0'));
        return InterFace { size: (1, 1), position: (0, 0), interface }
    }
}

impl Default for Border {
    fn default() -> Self {
        return Border::None
    }
}

// InterFaceWindow
pub struct Window<I> where I: InterFaceChild {
    child: Option<I>,
    interface: InterFace,
}

impl<I> Window<I> where I: InterFaceChild {
    pub fn new() -> Self {
        return Window { child: None, interface: InterFace::default() }
    }
    pub fn run(&self) -> std::io::Result<()> {
        print!("{}", self.interface);
        stdout().flush()?;
        return Ok(())
    }
    fn size(&self) -> (u16, u16) {
        if let Some(child) = &self.child {
            return child.size()
        }
        return (1, 1)
    }
}

impl<I> InterFaceParent<I> for Window<I> where I: InterFaceChild {
    fn attach(&mut self, widget: I) -> Result<(), ()> {
        self.child = Some(widget);
        return Ok(())
    }

    fn build(&mut self) {
        let size = self.size();
        if let Some(child) = &mut self.child {
            let interface_row = vec!('\0'; child.size().0 as usize);
            let interface = vec!(interface_row; child.size().1 as usize);
            self.interface = InterFace { size: child.size(), position: (0, 0), interface };

            // build child
            let child_if = child.build(size, (0, 0));
            self.interface.insert((0, 0), child_if);
        }
    }
}

pub struct Grid<I> where I: InterFaceChild {
    cells : Vec<GridCell<I>>,
    colums: u16,
    rows  : u16,
    interface: InterFace,
}

impl<I> InterFaceChild for Grid<I> where I: InterFaceChild {
    fn draw(&mut self) {
        todo!()
    }

    fn size(&self) -> (u16, u16) {
        let mut row_width_vec = vec![0, self.rows];
        let mut column_width_vec = vec![0, self.colums];
        for cell in &self.cells {
            let (x_size, y_size) = cell.size();
            let (x, y) = cell.coords();
            row_width_vec[y as usize] = max(row_width_vec[y as usize], y_size);
            column_width_vec[x as usize] = max(column_width_vec[x as usize], x_size)
        }
        return (column_width_vec.iter().sum::<u16>(), row_width_vec.iter().sum::<u16>() as u16)
    }

    fn build(&mut self, size: (u16, u16), position: (u16, u16)) -> &InterFace {
        todo!()
    }
}

struct GridCell<I> where I: InterFaceChild {
    column: u16,
    row: u16,
    child: Option<I>,
    interface: InterFace,
}

impl<I> GridCell<I> where I: InterFaceChild {
    fn new(column: u16, row: u16) -> Self {
        return GridCell { column, row, child: None, interface: InterFace::default() }
    }
    fn coords(&self) -> (u16, u16) {
        return (self.column, self.row)
    }
    fn draw(&self, _: String) {
        todo!()
    }

    fn size(&self) -> (u16, u16) {
        return match &self.child {
            Some(child) => { child.size() }
            None => (0, 0)
        }
    }
}

impl<I> InterFaceParent<I> for GridCell<I> where I: InterFaceChild {
    fn attach(&mut self, widget: I) -> Result<(), ()>  {
        self.child = Some(widget);
        Ok(())
    }

    fn build(&mut self) {
        todo!()
    }
}

pub struct EmptyWidget {}
impl InterFaceChild for EmptyWidget {
    fn draw(&mut self) {
        todo!()
    }

    fn size(&self) -> (u16, u16) {
        todo!()
    }

    fn build(&mut self, size: (u16, u16), position: (u16, u16)) -> &InterFace {
        todo!()
    }
}

pub struct Frame<I> where I: InterFaceChild {
    pub size: (u16, u16),
    pub child: Option<I>,
    border: Border,
    interface: InterFace,
}

impl<I> Frame<I> where I: InterFaceChild {
    pub fn new(size: (u16, u16)) -> Self {
        return Frame { size, child: None, border: Border::None, interface: InterFace::default() }
    }
    pub fn set_border(&mut self, border: Border) {
        self.border = border;
    }
    pub fn set_size(&mut self, size: (u16, u16)) {
        self.size = size
    }
}

impl<I> InterFaceChild for Frame<I> where I: InterFaceChild {
    fn draw(&mut self) {
        let size = self.size;
        match self.border {
            Border::Full => {
                let mut interface = self.interface.interface.clone();
                interface[0] = vec!('─'; self.size().0 as usize);
                interface[(size.1 - 1) as usize] = vec!('─'; self.size().0 as usize);
                for line in &mut interface {
                    line[0] = '│';
                    line[(self.size().0 - 1) as usize] = '│'
                }

                // draw corners
                interface[0][0] = '┌';
                interface[0][(self.size.0 - 1) as usize] = '┐';
                interface[(self.size.1 - 1) as usize][0] = '└';
                interface[(self.size.1 - 1) as usize][(self.size.0 - 1) as usize] = '┘';

                self.interface.interface = interface;
            }
            _ => {}
        }
    }

    fn size(&self) -> (u16, u16) {
        return self.size
    }

    fn build(&mut self, size: (u16, u16), position: (u16, u16)) -> &InterFace {
        let interface_row = vec!('\0'; self.size.0 as usize);
        let interface = vec!(interface_row; self.size.1 as usize);
        self.interface = InterFace { size: self.size, position, interface };
        self.draw();

        if let Some(child) = &mut self.child {
            // build child
            match self.border {
                Border::Full => {
                    let child_if = child.build(size, (position.0 + 1, position.1 + 1));
                    self.interface.insert((position.0 + 1, position.1 + 1), child_if);
                }
                Border::None => {
                    let child_if = child.build(size, (position.0, position.1));
                    self.interface.insert((0, 0), child_if);
                }
            }
        }
        return &self.interface
    }
}

pub struct Label {
    label: String,
}

impl InterFaceChild for Label {
    fn draw(&mut self) {
        todo!()
    }

    fn size(&self) -> (u16, u16) {
        let label_lines = self.label.split('\n').collect::<Vec<&str>>();
        let v_size = label_lines.len();
        let mut h_size = label_lines[0].len();
        for line in 1..label_lines.len() {
            h_size = max(h_size, label_lines[line].len())
        }
        return (v_size as u16, h_size as u16)
    }

    fn build(&mut self, size: (u16, u16), position: (u16, u16)) -> &InterFace {
        todo!()
    }
}
