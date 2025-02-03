mod command;
mod shell_io;

use shell_io::Output;

use std::cmp::PartialEq;
use std::env;
use std::env::set_current_dir;
use std::fmt::Write as _;
#[allow(unused_imports)]
use std::fs::File;
use std::io::{self, Write as _};
use std::iter::Peekable;
use std::path::Path;
use std::process::Command;
use std::str::Chars;

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
    Redirect(Output),
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
    fn append_token(&mut self, other: &String) {
        match self {
            Self::Literal(s) => s.push_str(other),
            _ => panic!("Only literals can be appended"),
        }
    }
    fn as_str(&self) -> &str {
        match self {
            Self::Literal(s) => s.as_str(),
            Self::Quote(q) => q.as_str(),
            Self::Redirect(r) => r.as_str(),
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            Self::Literal(s) => s.is_empty(),
            _ => false,
        }
    }
    fn parse(chars: &mut Peekable<Chars>) -> Self {
        let mut arg = Token::new();
        let mut quoting: Option<Quoting> = None;

        while let Some(c) = chars.peek() {
            let c = *c;

            let quote = Quoting::parse(c);

            let is_quoted = match (quoting, quote) {
                // Single escaped character
                (Some(Quoting::Escape), _) => {
                    arg.push(c);
                    quoting = None;
                    true
                }
                // Middle of quote
                (Some(Quoting::Quote | Quoting::DoubleQuote), None) => {
                    arg.push(c);
                    true
                }
                // Escape in the middle of a double quote
                (Some(Quoting::DoubleQuote), Some(Quoting::Escape)) => {
                    // clear current
                    chars.next();

                    let Some(next) = chars.peek() else {
                        return arg;
                    };

                    // if a special character then don't add escape
                    match next {
                        '$' | '\\' | '"' => {
                            arg.push(*next);
                        }
                        _ => {
                            arg.push(c);
                            arg.push(*next);
                        }
                    }

                    true
                }
                // End Quote
                (Some(Quoting::Quote), Some(Quoting::Quote))
                | (Some(Quoting::DoubleQuote), Some(Quoting::DoubleQuote)) => {
                    quoting = None;
                    true
                }
                // Start Quote
                (None, Some(_)) => {
                    quoting = quote;
                    true
                }
                // Non Quote logic
                (None, None) | (Some(_), Some(_)) => false,
            };

            if is_quoted {
                chars.next();
                continue;
            }

            let (consume_char, exit) = match c {
                // End arg
                ' ' => {
                    if arg.is_empty() {
                        (true, true)
                    } else {
                        (false, true)
                    }
                }
                // Redirect
                '1' | '2' => {
                    if !arg.is_empty() {
                        let mut dup_chars = chars.clone();
                        let next_token = Token::parse(&mut dup_chars);
                        match next_token {
                            // not a redirect so add to token and continue
                            Token::Literal(lit) => {
                                arg.append_token(&lit);
                                *chars = dup_chars;
                                (false, false)
                            }
                            // is the start of a new token so return value without consuming
                            _ => (false, true),
                        }
                    } else {
                        chars.next();
                        arg.push(c);
                        let Some(next) = chars.peek() else {
                            continue;
                        };

                        // if a special character then don't add escape
                        match (c, next) {
                            ('1', '>') => {
                                arg = Token::Redirect(Output::Stdout);
                                (true, true)
                            }
                            ('2', '>') => {
                                arg = Token::Redirect(Output::Stderr);
                                (true, true)
                            }
                            _ => (false, false),
                        }
                    }
                }
                '>' => {
                    if arg.is_empty() {
                        arg = Token::Redirect(Output::Stdout);
                        (true, true)
                    } else {
                        (false, true)
                    }
                }
                _ => {
                    arg.push(c);
                    (true, false)
                }
            };

            if consume_char {
                _ = chars.next();
            }
            if exit {
                return arg;
            }
        }
        arg
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

        let mut chars = input.chars().peekable();

        let mut tokens: Vec<_> = Vec::new();

        loop {
            let arg = Token::parse(&mut chars);

            if !arg.is_empty() {
                tokens.push(arg);
            }
            if chars.peek().is_none() {
                break;
            }
        }

        let mut command: Option<Token> = None;
        let mut args: Vec<_> = Vec::new();
        let mut output: Option<Output> = None;
        let mut errout: Option<Output> = None;

        let mut tokens = tokens.into_iter();

        while let Some(token) = tokens.next() {
            match (&token, &command) {
                (Token::Literal(_), None) => command = Some(token),
                (Token::Redirect(out), _) => {
                    let Some(next) = tokens.next() else {
                        continue;
                    };
                    let Token::Literal(path) = next else {
                        continue;
                    };
                    let file = Output::new_file(&path).ok();
                    match out {
                        Output::Stdout => output = file,
                        Output::Stderr => errout = file,
                        _ => (),
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
