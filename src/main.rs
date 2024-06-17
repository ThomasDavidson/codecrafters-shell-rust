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

        let first_line = input.lines().next().unwrap();

        let (command, argument) = first_line.split_once(" ").unwrap_or_else(|| (first_line, ""));


        match command {
            "exit" => break,
            "echo" => println!("{}", argument),
            "type" => {
                if ["exit"
                    , "echo"
                    , "type"].contains(&argument) {
                    println!("exit is a shell builtin")
                } else {
                    println!("{}: command not found", argument)
                }
            }
            _ => println!("{}: command not found", command),
        };
    }
}
