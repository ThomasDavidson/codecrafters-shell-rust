use std::env;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn get_path() -> Vec<String> {
    let key = "PATH";
    match env::var_os(key) {
        Some(paths) => {
            env::split_paths(&paths).map(|p| p.to_str().unwrap().to_string()).collect()
        }
        None => Vec::new()
    }
}

fn file_on_path(file: &str) -> Option<String> {
    let paths = get_path();
    for path in paths {
        let file_check = format!("{}/{}", path, file);
        if Path::new(&file_check).exists() {
            return Some(file_check);
        }
    }

    None
}


fn main() {
    get_path();
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
                    println!("{} is a shell builtin", argument)
                } else if let Some(file) = file_on_path(argument) {
                    println!("{} is {}", argument, file)
                } else {
                    println!("{}: not found", argument)
                }
            }
            _ => {
                if let Some(file) = file_on_path(argument) {
                    let output = Command::new("sh")
                        .arg(first_line)
                        .output()
                        .unwrap();
                    let fmt_output = output.stdout.into_iter().map(|c| c as char).collect::<String>();
                    println!("Hello {}! The secret code is {}", argument, fmt_output);
                } else {
                    println!("{}: command not found", command)
                }
            }
        };
    }
}
