use std::io;

pub mod header;
pub mod r#impl;
pub mod key_matcher;
pub mod theme;

pub struct Printer<W> {
    writer: W,
    indent: usize,
}

impl<W> Printer<W> {
    pub fn new(writer: W) -> Self {
        Self { writer, indent: 0 }
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        if self.indent == 0 {
            panic!("Cannot dedent - indent was 0");
        }
        self.indent -= 1;
    }
}

impl<W> Printer<W>
where
    W: io::Write,
{
    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        self.begin_line()?;
        self.write(line)?;
        writeln!(self.writer)
    }

    pub fn begin_line(&mut self) -> io::Result<()> {
        let tabs = [b'\t'; 1];
        for _ in 0..self.indent {
            self.writer.write_all(&tabs)?;
        }
        Ok(())
    }

    pub fn write(&mut self, s: &str) -> io::Result<()> {
        self.writer.write_all(s.as_bytes())?;
        Ok(())
    }

    pub fn write_fmt(&mut self, args: std::fmt::Arguments) -> io::Result<()> {
        self.begin_line()?;
        self.writer.write_fmt(args)?;
        Ok(())
    }
}
