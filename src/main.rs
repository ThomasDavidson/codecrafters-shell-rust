mod shell_io;

use shell_io::Output;

use crate::shell_io::{RedirectFrom, WriteType};
use std::cmp::PartialEq;
use std::env;
use std::env::set_current_dir;
use std::fmt::Write as _;
#[allow(unused_imports)]
use std::fs::File;
use std::io::{self, Write as _};
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

#[derive(Debug)]
enum Token {
    Literal(String),
    Redirect(WriteType, RedirectFrom),
    #[allow(dead_code)]
    Quote(Quoting),
}
impl Token {
    fn new() -> Self {
        Self::Literal(String::new())
    }
    fn push(&mut self, c: char) {
        match self {
            Self::Literal(s) => s.push(c),
            _ => panic!("Only literals can be pushed"),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Literal(s) => s.as_str(),
            Self::Quote(q) => q.as_str(),
            Self::Redirect(_, _) => ">",
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            Self::Literal(s) => s.is_empty(),
            _ => false,
        }
    }
    fn parse(input: &str) -> Vec<Self> {
        let mut tokens: Vec<_> = Vec::new();
        let mut token = Token::new();
        let mut quoting: Option<Quoting> = None;

        // add padding
        let mut buf = input.to_string();
        buf.push_str("   ");

        let mut windows = buf.as_bytes().windows(3);
        while let Some(window) = windows.next() {
            let (c1, c2, c3) = (window[0] as char, window[1] as char, window[2] as char);

            let quote = Quoting::parse(c1);

            let mut is_quoted = true;
            match (quoting, quote) {
                // Single escaped character
                (Some(Quoting::Escape), _) => {
                    token.push(c1);
                    quoting = None;
                }
                // Middle of quote
                (Some(Quoting::Quote | Quoting::DoubleQuote), None) => {
                    token.push(c1);
                }
                // Escape in the middle of a double quote
                (Some(Quoting::DoubleQuote), Some(Quoting::Escape)) => {
                    // clear next since it gets pushed
                    _ = windows.next();

                    // if a special character then don't add escape
                    match c2 {
                        '$' | '\\' | '"' => {
                            token.push(c2);
                        }
                        _ => {
                            token.push(c1);
                            token.push(c2);
                        }
                    }
                }
                // End Quote
                (Some(Quoting::Quote), Some(Quoting::Quote))
                | (Some(Quoting::DoubleQuote), Some(Quoting::DoubleQuote)) => {
                    quoting = None;
                }
                // Start Quote
                (None, Some(_)) => {
                    quoting = quote;
                }
                // Non Quote logic
                (None, None) | (Some(_), Some(_)) => is_quoted = false,
            };

            if is_quoted {
                continue;
            }

            match (c1, c2, c3) {
                // End arg
                (' ', _, _) => {
                    if !token.is_empty() {
                        tokens.push(token);
                        token = Token::new();
                    }
                }
                // Redirect
                ('1' | '2', '>', _) | ('>', _, _) => {
                    if !token.is_empty() {
                        tokens.push(token);
                        token = Token::new();
                    }

                    let redirect_type = match (c1, c2, c3) {
                        // 1>> 2>> >> 
                        ('>', '>', _) | ('1' | '2', '>', '>') => WriteType::Append,
                        // >
                        _ => WriteType::Overwrite,
                    };

                    // skip read tokens
                    let read_tokens = match (c1, c2, c3) {
                        (_, '>', '>') => 3,
                        (_, '>', _) => 2,
                        _ => 1,
                    };
                    for _ in 0..read_tokens {
                        windows.next();
                    }

                    match c1 {
                        '1' | '>' => {
                            token = Token::Redirect(redirect_type, RedirectFrom::Stdout);
                        }
                        '2' => {
                            token = Token::Redirect(redirect_type, RedirectFrom::Stderr);
                        }
                        _ => (),
                    }
                    tokens.push(token);
                    token = Token::new();
                }
                _ => {
                    token.push(c1);
                }
            };
        }

        if !token.is_empty() {
            tokens.push(token);
        }

        tokens
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum Quoting {
    Quote,
    DoubleQuote,
    Escape,
}
impl Quoting {
    fn parse(c: char) -> Option<Quoting> {
        match c {
            '\'' => Some(Self::Quote),
            '\"' => Some(Self::DoubleQuote),
            '\\' => Some(Self::Escape),
            _ => None,
        }
    }
    fn as_str(&self) -> &str {
        match self {
            Self::Quote => "\'",
            Self::DoubleQuote => "\"",
            Self::Escape => "\\",
        }
    }
}

#[test]
fn test_double_quote_1() {
    let input = r#"echo "hello'script'\\n'world""#.to_string();
    let result = ShellExec::parse(&input);
    assert_eq!(result.command.as_str(), "echo");
    assert_eq!(result.get_args(), [r#"hello'script'\n'world"#]);
}
#[test]
fn test_double_quote_2() {
    let input = r#"echo "hello\"insidequotes"script\"#.to_string();
    let result = ShellExec::parse(&input);
    assert_eq!(result.command.as_str(), "echo");
    assert_eq!(result.get_args(), [r#"hello"insidequotesscript"#]);
}
#[derive(Debug)]
struct ShellExec {
    command: Token,
    args: Vec<Token>,
    output: Output,
    errout: Output,
}

impl ShellExec {
    fn parse(input: &String) -> Self {
        let input = input.trim();

        let tokens = Token::parse(input);

        let mut command: Option<Token> = None;
        let mut args: Vec<_> = Vec::new();
        let mut output: Option<Output> = None;
        let mut errout: Option<Output> = None;

        let mut tokens = tokens.into_iter();

        while let Some(token) = tokens.next() {
            match (&token, &command) {
                (Token::Literal(_), None) => command = Some(token),
                (Token::Redirect(write_type, output_type), _) => {
                    let Some(next) = tokens.next() else {
                        continue;
                    };
                    let Token::Literal(path) = next else {
                        continue;
                    };
                    let file = Output::new_file(&path, write_type).ok();
                    match output_type {
                        RedirectFrom::Stdout => output = file,
                        RedirectFrom::Stderr => errout = file,
                    }

                    // stop checking tokens after full command for now
                    break;
                }
                _ => args.push(token),
            }
        }

        Self {
            command: command.unwrap_or(Token::new()),
            args,
            output: output.unwrap_or(Output::Stdout),
            errout: errout.unwrap_or(Output::Stderr),
        }
    }
    #[allow(dead_code)]
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
        _ = write!(std::io::stdout(), "$ ");

        io::stdout().flush().unwrap();

        // Wait for user input
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();

        let mut shell_exec = ShellExec::parse(&input);

        let command = shell_exec.command.as_str();

        let mut output = String::new();
        let mut errout = String::new();

        match command {
            "exit" => break,
            "echo" => {
                for arg in shell_exec.get_args() {
                    _ = write!(&mut output, "{arg} ")
                }
            }
            "type" => {
                if ["exit", "echo", "type", "pwd"].contains(&shell_exec.get_arg()) {
                    _ = write!(&mut output, "{} is a shell builtin", shell_exec.get_arg());
                } else if let Some(file) = file_on_path(shell_exec.get_arg()) {
                    _ = write!(&mut output, "{} is {}", shell_exec.get_arg(), file);
                } else {
                    _ = write!(&mut errout, "{}: not found", shell_exec.get_arg());
                }
            }
            "pwd" => {
                match env::current_dir() {
                    Ok(path) => _ = write!(&mut output, "{}", path.display()),
                    Err(e) => {
                        _ = write!(&mut errout, "Error: {:?}", e);
                        continue;
                    }
                };
            }
            "cd" => {
                let path: Option<String> = if shell_exec.get_arg().contains("~") {
                    if let Some(home) = get_home() {
                        Some(shell_exec.get_arg().replace("~", &home))
                    } else {
                        _ = write!(
                            &mut errout,
                            "cd: {}: No such file or directory",
                            shell_exec.get_arg()
                        );
                        None
                    }
                } else {
                    Some(shell_exec.get_arg().to_string())
                };

                match path {
                    Some(path) => match set_current_dir(path) {
                        Ok(_) => (),
                        Err(_) => {
                            _ = write!(
                                &mut errout,
                                "cd: {}: No such file or directory",
                                shell_exec.get_arg()
                            );
                        }
                    },
                    None => (),
                }
            }
            _ => {
                if let Some(_) = file_on_path(command) {
                    let cmd_output = Command::new("sh").arg("-c").arg(input).output().unwrap();

                    let fmt_output = cmd_output
                        .stdout
                        .into_iter()
                        .map(|c| c as char)
                        .collect::<String>();
                    _ = write!(&mut output, "{}", fmt_output.trim_end());

                    let fmt_err = cmd_output
                        .stderr
                        .into_iter()
                        .map(|c| c as char)
                        .collect::<String>();
                    _ = write!(&mut errout, "{}", fmt_err.trim_end());
                } else {
                    _ = write!(&mut errout, "{}: command not found", command)
                }
            }
        };

        if !output.is_empty() {
            shell_exec.output.write(output);
        }

        if !errout.is_empty() {
            shell_exec.errout.write(errout);
        }
    }
}
