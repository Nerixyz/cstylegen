use std::{
    ffi::OsStr,
    io::{stderr, Write},
};

use cssparser::{BasicParseErrorKind, SourceLocation};

use crate::parse;

pub fn print_error_with_source(
    source_id: &OsStr,
    source: &str,
    message: &str,
    location: &SourceLocation,
) {
    if !try_print_error_with_source(source_id, source, message, location) {
        print_message_and_loc(source_id, message, location);
    }
}

pub fn format_css_parse_error(
    e: &cssparser::ParseError<parse::ParseError>,
) -> String {
    match &e.kind {
        cssparser::ParseErrorKind::Basic(
            BasicParseErrorKind::AtRuleBodyInvalid,
        ) => "@-rule body is invalid".to_owned(),
        cssparser::ParseErrorKind::Basic(
            BasicParseErrorKind::AtRuleInvalid(s),
        ) => format!("Invalid @-rule ({s})"),
        cssparser::ParseErrorKind::Basic(BasicParseErrorKind::EndOfInput) => {
            "Unexpected end of input".to_owned()
        }
        cssparser::ParseErrorKind::Basic(
            BasicParseErrorKind::QualifiedRuleInvalid,
        ) => "Qualified rule is invalid".to_owned(),
        cssparser::ParseErrorKind::Basic(
            BasicParseErrorKind::UnexpectedToken(t),
        ) => format!("Unexpected token ({t:?})"),
        cssparser::ParseErrorKind::Custom(p) => p.to_string(),
    }
}

fn try_print_error_with_source(
    source_id: &OsStr,
    source: &str,
    message: &str,
    location: &SourceLocation,
) -> bool {
    let Some(prev_line) = source.bytes().enumerate().filter(|&(_, x)| x == b'\n').map(|(i,_)| i).nth(location.line.saturating_sub(2) as usize) else {
            return false;
        };
    if source.len() - prev_line < 3 {
        return false;
    }
    let start = &source[prev_line + 1..];
    let (Some(prev_line_end), Some(err_line_end)) = ({
        let mut it = start.bytes().enumerate().filter(|&(_, x)| x == b'\n').map(|(i,_)| i);
        let first = it.next();
        let second = it.next();
        (first, second)
    }) else {
        return false;
    };

    let err_line_end = fix_clrf(start, err_line_end);
    let current_line = &start[prev_line_end + 1..err_line_end];
    let prev_line_end = fix_clrf(start, prev_line_end);

    eprintln!("{}:", source_id.to_string_lossy());
    eprintln!("{:>5}│ {}", location.line - 1, &start[..prev_line_end]);
    eprintln!("{:>5}│ {}", location.line, current_line);
    let mut stderr = stderr().lock();
    for _ in 0..(5 + 2 + location.column - 1) {
        stderr.write_all(&[b' ']).ok();
    }
    writeln!(stderr, "╰─► {message}").ok();

    true
}

fn print_message_and_loc(
    source_id: &OsStr,
    message: &str,
    location: &SourceLocation,
) {
    eprintln!(
        "[{} @ line {}, column {}] {message}",
        source_id.to_string_lossy(),
        location.line,
        location.column
    );
}

fn fix_clrf(source: &str, pos: usize) -> usize {
    if pos > 1 && source.as_bytes()[pos - 2] == b'\r' {
        pos - 1
    } else {
        pos
    }
}
