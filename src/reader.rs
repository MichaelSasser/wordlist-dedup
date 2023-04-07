use std::{
    fs::File,
    io::{
        self,
        prelude::*,
    },
    rc::Rc,
};

const LF: u8 = b'\n';

pub struct BufReader {
    reader: io::BufReader<File>,
    buf: Rc<String>,
}

impl BufReader {
    pub fn open(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        let buf = new_buf();

        Ok(Self { reader, buf })
    }
}

impl Iterator for BufReader {
    type Item = io::Result<Rc<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        let buf = match Rc::get_mut(&mut self.buf) {
            Some(buf) => {
                buf.clear();
                buf
            },
            None => {
                self.buf = new_buf();
                Rc::make_mut(&mut self.buf)
            },
        };

        self.reader
            .read_line(buf)
            .map(|u| {
                if u == 0 {
                    None
                } else {
                    Some(Rc::clone(&self.buf))
                }
            })
            .transpose()
    }
}

pub trait SizeOf {
    fn size_of(&mut self) -> Result<u64, io::Error>;
}

impl SizeOf for BufReader {
    fn size_of(&mut self) -> Result<u64, io::Error> {
        let mut counter: u64 = 0;
        let mut line: Vec<u8> = Vec::new();
        while match self.reader.read_until(LF, &mut line) {
            Ok(n) if n > 0 => true,
            Err(e) => return Err(e),
            _ => false,
        } {
            if *line.last().unwrap() == LF {
                counter += 1;
            };
        }
        Ok(counter)
    }
}

fn new_buf() -> Rc<String> {
    Rc::new(String::with_capacity(1024))
}
