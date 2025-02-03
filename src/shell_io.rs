use std::cmp::PartialEq;
#[allow(unused_imports)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Write as _};

#[derive(Debug)]
pub enum RedirectFrom {
    Stdout,
    Stderr,
}

#[derive(Debug, PartialEq)]
pub enum WriteType {
    Append,
    Overwrite,
}

#[derive(Debug)]
pub enum Output {
    File(File),
    Stdout,
    Stderr,
}
impl Output {
    pub fn write(&mut self, content: String) {
        match self {
            Self::File(file) => _ = writeln!(file, "{}", content).unwrap(),
            Self::Stdout => _ = writeln!(io::stdout(), "{}", content),
            Self::Stderr => _ = writeln!(io::stderr(), "{}", content),
        }
    }
    pub fn new_file(path: &String, write_type: &WriteType) -> io::Result<Self> {
        match write_type {
            WriteType::Append => {
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(path)?;
                Ok(Self::File(file))
            }
            WriteType::Overwrite => {
                let file = OpenOptions::new().create(true).write(true).open(path)?;

                Ok(Self::File(file))
            }
        }
    }
}
