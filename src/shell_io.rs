#[allow(unused_imports)]
use std::fs::File;
use std::io::{self, Write as _};
use std::path::Path;

#[derive(Debug)]
pub enum Output {
    File(File),
    Stdout,
}
impl Output {
    pub fn write(&mut self, content: String) {
        match self {
            Self::File(file) => _ = file.write(content.as_bytes()).unwrap(),
            Self::Stdout => _ = writeln!(io::stdout(), "{}", content),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Stdout => "",
            Self::File(_) => "1>",
        }
    }
    pub fn new_file(path: &String) -> io::Result<Self> {
        let file = File::create(Path::new(&path))?;

        Ok(Self::File(file))
    }
}
