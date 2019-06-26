use std::fmt::Write;
use std::io::{self, BufRead};
use std::path::Path;

use console::strip_ansi_codes;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub const DELTA_THEME_DEFAULT: &str = "InspiredGitHub"; // base16-mocha.dark

const GREEN: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0x00,
};

const RED: Color = Color {
    r: 0xff,
    g: 0xd0,
    b: 0xd0,
    a: 0x00,
};

#[derive(PartialEq)]
enum State {
    Commit,
    DiffMeta,
    DiffHunk,
    Unknown,
}

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes[DELTA_THEME_DEFAULT];
    let mut output = String::new();
    let mut state = State::Unknown;
    let mut syntax: Option<&SyntaxReference> = None;
    let mut have_printed_line: bool;

    for _line in io::stdin().lock().lines() {
        let raw_line = _line.unwrap(); // TODO: handle None
        let line = strip_ansi_codes(&raw_line).to_string();
        have_printed_line = false;
        if line.starts_with("diff --") {
            state = State::DiffMeta;
            syntax = match get_file_extension_from_diff_line(&line) {
                Some(extension) => ps.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::DiffHunk;
        } else if state == State::DiffHunk {
            match syntax {
                Some(syntax) => {
                    let mut highlighter = HighlightLines::new(syntax, theme);
                    let background_color = match line.chars().next() {
                        Some('+') => Some(GREEN),
                        Some('-') => Some(RED),
                        _ => None,
                    };
                    let line_without_plus_minus = if line.len() > 0 { &line[1..] } else { &line };
                    let ranges: Vec<(Style, &str)> =
                        highlighter.highlight(line_without_plus_minus, &ps);
                    my_as_24_bit_terminal_escaped(&ranges[..], background_color, &mut output);
                    println!(" {}", output);
                    output.truncate(0);
                    have_printed_line = true;
                }
                None => (),
            }
        }
        if !have_printed_line {
            println!("{}", raw_line);
        }
    }
}

/// Based on as_24_bit_terminal_escaped from syntect
fn my_as_24_bit_terminal_escaped(
    v: &[(Style, &str)],
    background_color: Option<Color>,
    buf: &mut String,
) -> () {
    for &(ref style, text) in v.iter() {
        colorize(text, Some(style.foreground), background_color, false, buf);
    }
    buf.push_str("\x1b[0m");
}

/// Write text to buffer with color escape codes applied.
fn colorize(
    text: &str,
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    reset_color: bool,
    buf: &mut String,
) -> () {
    match background_color {
        Some(background_color) => {
            write!(
                buf,
                "\x1b[48;2;{};{};{}m",
                background_color.r,
                background_color.g,
                background_color.b
            ).unwrap();
            if reset_color {
                buf.push_str("\x1b[0m");
            }
        }
        None => (),
    }
    match foreground_color {
        Some(foreground_color) => {
            write!(
                buf,
                "\x1b[38;2;{};{};{}m{}",
                foreground_color.r,
                foreground_color.g,
                foreground_color.b,
                text
            ).unwrap();
            if reset_color {
                buf.push_str("\x1b[0m");
            }
        }
        None => {
            write!(buf, "{}", text).unwrap();
        }
    }
}

/// Given input like
/// "diff --git a/src/main.rs b/src/main.rs"
/// Return "rs", i.e. a single file extension consistent with both files.
fn get_file_extension_from_diff_line(line: &str) -> Option<&str> {
    match get_file_extensions_from_diff_line(line) {
        (Some(ext1), Some(ext2)) => {
            if ext1 == ext2 {
                Some(ext1)
            } else {
                // TODO: Just return None and output without color.
                panic!(
                    "Old and new files have different extensions: {} vs {}",
                    ext1,
                    ext2
                );
            }
        }
        (Some(ext1), None) => Some(ext1),
        (None, Some(ext2)) => Some(ext2),
        (None, None) => None,
    }
}

/// Given input like "diff --git a/src/main.rs b/src/main.rs"
/// return ("src/main.rs", "src/main.rs").
fn get_file_extensions_from_diff_line(line: &str) -> (Option<&str>, Option<&str>) {
    let mut iter = line.split(" ");
    iter.next(); // diff
    iter.next(); // --git
    (
        iter.next().and_then(|s| get_extension(&s[2..])),
        iter.next().and_then(|s| get_extension(&s[2..])),
    )
}

/// Attempt to parse input as a file path and return extension as a &str.
fn get_extension(s: &str) -> Option<&str> {
    Path::new(s).extension().and_then(|e| e.to_str())
}
