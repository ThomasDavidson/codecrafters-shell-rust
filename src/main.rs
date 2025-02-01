use std::env;
use std::env::set_current_dir;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn get_path() -> Vec<String> {
    let key = "PATH";
    match env::var_os(key) {
        Some(paths) => env::split_paths(&paths)
            .map(|p| p.to_str().unwrap().to_string())
            .collect(),
        None => Vec::new(),
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

fn get_home() -> Option<String> {
    let key = "HOME";
    match env::var_os(key) {
        Some(path) => {
            let Ok(str_path) = path.into_string() else {
                return None;
            };
            Some(str_path)
        }
        None => None,
    }
}

struct ShellExec {
    command: String,
    args: Vec<String>,
}
impl ShellExec {
    fn parse(input: &String) -> Self {
        let (command, arg) = input
            .trim()
            .split_once(" ")
            .unwrap_or_else(|| (input.trim(), ""));

        let args = arg
            .split("'")
            .filter(|s| s.len() > 0)
            .map(|s| s.to_string())
            .collect();

        Self {
            command: command.to_string(),
            args,
        }
    }
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

        let shell_exec = ShellExec::parse(&input);

        match shell_exec.command.as_str() {
            "exit" => break,
            "echo" => {
                for arg in shell_exec.args {
                    print!("{arg}")
                }
                println!();
            }

            "type" => {
                if ["exit", "echo", "type", "pwd"].contains(&shell_exec.args[0].as_str()) {
                    println!("{} is a shell builtin", shell_exec.args[0])
                } else if let Some(file) = file_on_path(shell_exec.args[0].as_str()) {
                    println!("{} is {}", shell_exec.args[0], file)
                } else {
                    println!("{}: not found", shell_exec.args[0])
                }
            }
            "pwd" => {
                let path = match env::current_dir() {
                    Ok(t) => t,
                    Err(e) => {
                        println!("Error: {:?}", e);
                        continue;
                    }
                };
                println!("{}", path.display());
            }
            "cd" => {
                let path = if shell_exec.args[0].contains("~") {
                    let Some(home) = get_home() else {
                        println!("cd: {}: No such file or directory", shell_exec.args[0]);
                        continue;
                    };
                    shell_exec.args[0].replace("~", &home)
                } else {
                    shell_exec.args[0].to_string()
                };

                match set_current_dir(path) {
                    Ok(_) => continue,
                    Err(_) => println!("cd: {}: No such file or directory", shell_exec.args[0]),
                }
            }
            command => {
                if let Some(_) = file_on_path(command) {
                    let output = Command::new("sh").arg("-c").arg(&input).output().unwrap();

                    let fmt_output = output
                        .stdout
                        .into_iter()
                        .map(|c| c as char)
                        .collect::<String>();
                    println!("{}", fmt_output.trim());
                } else {
                    println!("{}: command not found", command)
                }
            }
        };
    }
}
