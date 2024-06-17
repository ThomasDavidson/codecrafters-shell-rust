#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        // Wait for user input
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();

        let command = input.lines().next().unwrap();

        match command {
            "exit 0" => break,
            _ => println!("{}: command not found", command),
        };
    }
}
