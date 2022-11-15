use std::cmp::max;
use std::io::{stdout, Write};

pub trait InterFaceChild {
    fn draw(&self, _: String) -> String;
    fn size(&self) -> (u16, u16);
}

pub trait InterFaceParent<I> where I: InterFaceChild {
    fn attach(&mut self, widget: I) -> Result<(), ()> ;
}

pub enum Border {
    None,
    Full,
}
impl Default for Border {
    fn default() -> Self {
        return Border::None
    }
}

// InterFaceWindow
pub struct InterFaceWindow<I> where I: InterFaceChild {
    child: Option<I>,
}

impl<I> InterFaceWindow<I> where I: InterFaceChild {
    pub fn new() -> Self {
        return InterFaceWindow { child: None }
    }
    pub fn run(&self) -> std::io::Result<()> {
        if let Some(child) = &self.child {
            let mut row_string = "*".repeat(child.size().0 as usize);
            row_string.push('\n');
            let mut window_string = row_string.repeat(child.size().1 as usize);
            window_string = child.draw(window_string);
            print!("{}", window_string);
            stdout().flush()?;
            return Ok(())
        }
        return Ok(())
    }
}

impl<I> InterFaceParent<I> for InterFaceWindow<I> where I: InterFaceChild {
    fn attach(&mut self, widget: I) -> Result<(), ()> {
        self.child = Some(widget);
        return Ok(())
    }
}

pub struct InterFaceGrid<I> where I: InterFaceChild {
    cells : Vec<InterFaceGridCell<I>>,
    colums: u16,
    rows  : u16,
}

impl<I> InterFaceChild for InterFaceGrid<I> where I: InterFaceChild {
    fn draw(&self, _: String) -> String {
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
}

struct InterFaceGridCell<I> where I: InterFaceChild {
    column: u16,
    row: u16,
    child: Option<I>,
}

impl<I> InterFaceGridCell<I> where I: InterFaceChild {
    fn new(column: u16, row: u16) -> Self {
        return InterFaceGridCell { column, row, child: None }
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

impl<I> InterFaceParent<I> for InterFaceGridCell<I> where I: InterFaceChild {
    fn attach(&mut self, widget: I) -> Result<(), ()>  {
        self.child = Some(widget);
        Ok(())
    }
}

pub struct EmptyWidget {}
impl InterFaceChild for EmptyWidget {
    fn draw(&self, _: String) -> String {
        todo!()
    }

    fn size(&self) -> (u16, u16) {
        todo!()
    }
}

pub struct Frame<I> where I: InterFaceChild {
    pub size: (u16, u16),
    pub child: Option<I>,
    border: Border,
}

impl<I> Frame<I> where I: InterFaceChild {
    pub fn new(size: (u16, u16)) -> Self {
        return Frame { size, child: None, border: Border::None }
    }
    pub fn set_border(&mut self, border: Border) {
        self.border = border;
    }
}

impl<I> InterFaceChild for Frame<I> where I: InterFaceChild {
    fn draw(&self, mut interface: String) -> String {
        if let Some(child) = &self.child {
            interface = child.draw(interface);
        }
        match self.border {
            Border::Full => {
                let mut interface_vec = Vec::new();
                let mut interface_lines = interface.split('\n').to_owned().collect::<Vec<String>>();
                interface_lines[0] = format!("┌{}┐", "─".repeat((self.size.0 - 2) as usize));
                for line in interface_lines {
                    interface_vec.push(line.chars().collect::<Vec<char>>())
                }

                // draw corners
                // interface_vec[0..((self.size.0 - 1) as usize)] = '─'
                // interface_vec[0] = '┌';
                // interface_vec[(self.size.0 - 1) as usize] = '┐';
                // interface_vec[((self.size.0 + 1) * (self.size.1 - 1)) as usize] = '└';
                // interface_vec[((self.size.0 + 1) * self.size.1 - 2) as usize] = '┘';
                // interface = interface_vec.iter().collect::<String>()
            }
            _ => {}
        }
        return interface
    }

    fn size(&self) -> (u16, u16) {
        return self.size
    }
}
