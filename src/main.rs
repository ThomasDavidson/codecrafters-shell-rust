use std::cmp::PartialEq;
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

#[derive(Debug, PartialEq, Copy, Clone)]
enum Quoting {
    Quote,
    DoubleQuote(bool),
    Escape,
}
impl Quoting {
    fn parse(c: char) -> Option<Quoting> {
        match c {
            '\'' => Some(Self::Quote),
            '\"' => Some(Self::DoubleQuote(false)),
            '\\' => Some(Self::Escape),
            _ => None,
        }
    }
}

#[derive(PartialEq)]
struct ShellExec {
    command: String,
    args: Vec<String>,
}

impl ShellExec {
    fn parse(input: &String) -> Self {
        let input = input.trim();

        let mut args: Vec<_> = Vec::new();
        let mut arg = String::new();

        let mut quoting: Option<Quoting> = None;

        for c in input.chars() {
            let quote = Quoting::parse(c);

            match (quoting, quote) {
                // Single escaped character
                (Some(Quoting::Escape), _) => {
                    arg.push(c);
                    quoting = None;
                }
                // Middle of quote
                (Some(Quoting::Quote | Quoting::DoubleQuote(false)), None) => arg.push(c),
                (Some(Quoting::DoubleQuote(false)), Some(Quoting::Escape)) => {
                    quoting = Some(Quoting::DoubleQuote(true))
                }
                // Escape in the middle of a double quote
                (Some(Quoting::DoubleQuote(true)), _) => {
                    // if a special character then don't add escape
                    match c {
                        '$' | '\\' | '"' => {
                            arg.push(c);
                        }
                        _ => {
                            arg.push('\\');
                            arg.push(c)
                        }
                    }
                    quoting = Some(Quoting::DoubleQuote(false))
                }
                // End Quote
                (Some(Quoting::Quote), Some(Quoting::Quote))
                | (Some(Quoting::DoubleQuote(_)), Some(Quoting::DoubleQuote(_))) => {
                    quoting = None;
                    continue;
                }
                // Start Quote
                (None, Some(_)) => quoting = quote,
                // Non Quote logic
                (None, None) | (Some(_), Some(_)) => match c {
                    ' ' => {
                        if !arg.is_empty() {
                            args.push(arg.clone());
                            arg.clear();
                        }
                    }
                    _ => arg.push(c),
                },
            }
        }
        if !arg.is_empty() {
            args.push(arg);
        }

        let command = if args.len() > 0 {
            args.remove(0)
        } else {
            String::new()
        };

        Self { command, args }
    }
    fn get_args(&self) -> Vec<&str> {
        self.args.iter().map(|t| t.as_str()).collect()
    }
    fn get_arg(&self) -> &str {
        self.args[0].as_str()
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

        let command = shell_exec.command.as_str();

        match command {
            "exit" => break,
            "echo" => {
                for arg in shell_exec.get_args() {
                    print!("{arg} ")
                }
                println!();
            }

            "type" => {
                if ["exit", "echo", "type", "pwd"].contains(&shell_exec.get_arg()) {
                    println!("{} is a shell builtin", shell_exec.get_arg())
                } else if let Some(file) = file_on_path(shell_exec.get_arg()) {
                    println!("{} is {}", shell_exec.get_arg(), file)
                } else {
                    println!("{}: not found", shell_exec.get_arg())
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
                let path = if shell_exec.get_arg().contains("~") {
                    let Some(home) = get_home() else {
                        println!("cd: {}: No such file or directory", shell_exec.get_arg());
                        continue;
                    };
                    shell_exec.get_arg().replace("~", &home)
                } else {
                    shell_exec.get_arg().to_string()
                };

                match set_current_dir(path) {
                    Ok(_) => continue,
                    Err(_) => println!("cd: {}: No such file or directory", shell_exec.get_arg()),
                }
            }
            _ => {
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
