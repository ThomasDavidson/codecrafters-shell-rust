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

        let (command, argument) = input.trim().split_once(" ").unwrap_or_else(|| (input.trim(), ""));


        match command {
            "exit" => break,
            // "echo" => println!("{}", argument),
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
                if let Some(_) = file_on_path(command) {
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&input)
                        .output()
                        .unwrap();

                    let fmt_output = output.stdout.into_iter().map(|c| c as char).collect::<String>();
                    println!("{}", fmt_output.trim());
                } else {
                    println!("{}: command not found", command)
                }
            }
        };
    }
}
