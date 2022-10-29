use lazy_static::lazy_static;
use regex::Regex;

use std::collections::HashMap;
use std::fmt;
use std::iter::Iterator;
use std::string::ToString;

lazy_static! {
    static ref FILE_DIFF_HEADER: regex::Regex =
        Regex::new(r"^diff --git a/(?P<from_file>.*?)\s* b/(?P<to_file>.*?)\s*$").unwrap();
    static ref OLD_MODE_HEADER: regex::Regex = Regex::new(r"^old mode (?P<mode>\d+)$").unwrap();
    static ref NEW_MODE_HEADER: regex::Regex = Regex::new(r"^new mode (?P<mode>\d+)$").unwrap();
    static ref NEW_FILE_MODE_HEADER: regex::Regex = Regex::new(r"^new file mode (?P<mode>\d+)$").unwrap();
    static ref DELETED_FILE_MODE_HEADER: regex::Regex = Regex::new(r"^deleted file mode (?P<mode>\d+)$").unwrap();
    static ref INDEX_DIFF_HEADER: regex::Regex = Regex::new(r"^index (?P<from_blob>.*?)\.\.(?P<to_blob>.*?)(?: (?P<mode>\d+))?$").unwrap();
    static ref BINARY_DIFF: regex::Regex = Regex::new(r"Binary files (?P<from_file>.*) and (?P<to_file>.*) differ$").unwrap();
    static ref A_FILE_CHANGE_HEADER: regex::Regex = Regex::new(r"^--- (?:/dev/null|a/(?P<file>.*?)\s*)$").unwrap();
    static ref B_FILE_CHANGE_HEADER: regex::Regex = Regex::new(r"^\+\+\+ (?:/dev/null|b/(?P<file>.*?)\s*)$").unwrap();
    static ref CHUNK_HEADER: regex::Regex = Regex::new(r"^@@ -(?P<from_line_start>\d+)(?:,(?P<from_line_count>\d+))? \+(?P<to_line_start>\d+)(?:,(?P<to_line_count>\d+))? @@(?P<line>.*)$").unwrap();

    static ref LINE_DIFF: regex::Regex = Regex::new(r"^(?P<action>[-+ ])(?P<line>.*)$").unwrap();
    static ref NO_NEWLINE: regex::Regex = Regex::new(r"^\\ No newline at end of file$").unwrap();
    static ref RENAME_HEADER: regex::Regex = Regex::new(r"^similarity index (?P<rate>\d*)").unwrap();
    static ref RENAME_A_FILE: regex::Regex = Regex::new(r"^rename from (?P<from_file>.*?)").unwrap();
    static ref RENAME_B_FILE: regex::Regex = Regex::new(r"^rename to (?P<to_file>.*?)").unwrap();
}

#[derive(Debug)]
pub enum ParseError {
    Expected(String),
    LineParseError(usize, String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Expected(s) => write!(f, "{}", s),
            ParseError::LineParseError(n, s) => write!(f, "Line: {}: {}", n, s),
        }
    }
}

fn captures_to_map(re: &Regex, text: &str) -> HashMap<String, String> {
    let caps = re.captures(text).unwrap();
    re.capture_names()
        .flatten()
        .filter_map(|n| Some((n.to_string(), caps.name(n)?.as_str().to_string())))
        .collect()
}

type ParseR = (String, HashMap<String, String>);

fn parse_line(line: String, prev_state: &str) -> Result<ParseR, ParseError> {
    let states = [
        "start_of_file",
        "new_mode_header",
        "line_diff",
        "no_newline",
        "index_diff_header",
        "binary_diff",
        "rename_b_file",
    ];

    if states.contains(&prev_state) {
        if FILE_DIFF_HEADER.is_match(&line) {
            let mode = "file_diff_header".to_string();
            let captures = captures_to_map(&FILE_DIFF_HEADER, &line);
            return Ok((mode, captures));
        } else if prev_state == "start_of_file" {
            return Err(ParseError::Expected(
                "expected file diff header".to_string(),
            ));
        }
    }

    // "old mode {MODE}"
    if prev_state == "file_diff_header" {
        if OLD_MODE_HEADER.is_match(&line) {
            let mode = "old_mode_header".to_string();
            let captures = captures_to_map(&OLD_MODE_HEADER, &line);
            return Ok((mode, captures));
        }
    }

    // "new mode {MODE}"
    if prev_state == "old_mode_header" {
        if NEW_MODE_HEADER.is_match(&line) {
            let mode = "new_mode_header".to_string();
            let captures = captures_to_map(&NEW_MODE_HEADER, &line);
            return Ok((mode, captures));
        } else {
            return Err(ParseError::Expected("expected new_mode_header".to_string()));
        }
    }

    // "new file mode {MODE}"
    if prev_state == "file_diff_header" {
        if NEW_FILE_MODE_HEADER.is_match(&line) {
            let mode = "new_file_mode_header".to_string();
            let captures = captures_to_map(&NEW_FILE_MODE_HEADER, &line);
            return Ok((mode, captures));
        }
    }

    // "deleted file mode {MODE}"
    if prev_state == "file_diff_header" {
        if DELETED_FILE_MODE_HEADER.is_match(&line) {
            let mode = "deleted_file_mode_header".to_string();
            let captures = captures_to_map(&DELETED_FILE_MODE_HEADER, &line);
            return Ok((mode, captures));
        }
    }

    // "index {FROM_COMMIT} {TO_COMMIT} [{MODE}]"
    if [
        "rename_b_file",
        "file_diff_header",
        "new_mode_header",
        "new_file_mode_header",
        "deleted_file_mode_header",
    ]
    .contains(&prev_state)
    {
        if RENAME_HEADER.is_match(&line) {
            let mode = "rename_header".to_string();
            let captures = captures_to_map(&RENAME_HEADER, &line);
            return Ok((mode, captures));
        }

        if INDEX_DIFF_HEADER.is_match(&line) {
            let mode = "index_diff_header".to_string();
            let captures = captures_to_map(&INDEX_DIFF_HEADER, &line);
            return Ok((mode, captures));
        } else {
            return Err(ParseError::Expected(
                "expected index_diff_header".to_string(),
            ));
        }
    }

    if prev_state == "rename_header" {
        if RENAME_A_FILE.is_match(&line) {
            let mode = "rename_a_file".to_string();
            let captures = captures_to_map(&RENAME_A_FILE, &line);
            return Ok((mode, captures));
        }
    }

    if prev_state == "rename_a_file" {
        if RENAME_B_FILE.is_match(&line) {
            let mode = "rename_b_file".to_string();
            let captures = captures_to_map(&RENAME_B_FILE, &line);
            return Ok((mode, captures));
        }
    }

    // "Binary files {FROM_FILE} and {TO_FILE} differ"
    if prev_state == "index_diff_header" {
        if BINARY_DIFF.is_match(&line) {
            let mode = "binary_diff".to_string();
            let captures = captures_to_map(&BINARY_DIFF, &line);
            return Ok((mode, captures));
        }
    }

    // "--- {FILENAME}"
    if prev_state == "index_diff_header" {
        if A_FILE_CHANGE_HEADER.is_match(&line) {
            let mode = "a_file_change_header".to_string();
            let captures = captures_to_map(&A_FILE_CHANGE_HEADER, &line);
            return Ok((mode, captures));
        } else {
            return Err(ParseError::Expected(
                "expected a_file_change_header".to_string(),
            ));
        }
    }

    // "+++ {FILENAME}"
    if prev_state == "a_file_change_header" {
        if B_FILE_CHANGE_HEADER.is_match(line.as_str()) {
            let mode = "b_file_change_header".to_string();
            let captures = captures_to_map(&B_FILE_CHANGE_HEADER, &line);
            return Ok((mode, captures));
        } else {
            return Err(ParseError::Expected(
                "expected b_file_change_header".to_string(),
            ));
        }
    }

    // "@@ {?}[,{?}] {?}[,{?}] @@[{LINE}]"
    if ["b_file_change_header", "line_diff", "no_newline"].contains(&prev_state) {
        if CHUNK_HEADER.is_match(line.as_str()) {
            let mut captures = captures_to_map(&CHUNK_HEADER, &line);
            if !captures.contains_key("from_line_count") {
                captures.insert("from_line_count".to_string(), "1".to_string());
            }
            if !captures.contains_key("to_line_count") {
                captures.insert("to_line_count".to_string(), "1".to_string());
            }
            if !captures.contains_key("to_line_start") {
                let from_line_start = captures.get("from_line_start").unwrap();
                captures.insert("to_line_start".to_string(), from_line_start.to_string());
            }
            let from_line_start = captures.get("from_line_start").unwrap();
            captures.insert("from_line_start".to_string(), from_line_start.to_string());
            let from_line_count = captures.get("from_line_count").unwrap();
            captures.insert("from_line_count".to_string(), from_line_count.to_string());
            let to_line_start = captures.get("to_line_start").unwrap();
            captures.insert("to_line_start".to_string(), to_line_start.to_string());
            let to_line_count = captures.get("to_line_count").unwrap();
            captures.insert("to_line_count".to_string(), to_line_count.to_string());
            let mode = "chunk_header".to_string();
            return Ok((mode, captures));
        } else if prev_state == "b_file_change_header" {
            return Err(ParseError::Expected("expected chunk_header".to_string()));
        }
    }

    // "-{LINE}"
    // "+{LINE}"
    // " {LINE}"
    if ["chunk_header", "line_diff", "no_newline"].contains(&prev_state) {
        if LINE_DIFF.is_match(line.as_str()) {
            let mode = "line_diff".to_string();
            let captures = captures_to_map(&LINE_DIFF, &line);
            return Ok((mode, captures));
        }
    }

    // "\ No newline at end of file"
    if ["chunk_header", "line_diff"].contains(&prev_state) {
        if NO_NEWLINE.is_match(line.as_str()) {
            let mode = "NO_NEWLINE".to_string();
            let captures = captures_to_map(&NO_NEWLINE, &line);
            return Ok((mode, captures));
        } else {
            return Err(ParseError::Expected(
                "expected line_diff or no_newline".to_string(),
            ));
        }
    }

    return Err(ParseError::Expected(format!(
        "can't parse line with prev_state {:?}",
        prev_state
    )));
}

type ParsedLines = Vec<(String, HashMap<String, String>, String)>;

type ParseLinesResult = Result<ParsedLines, ParseError>;

pub fn parse_lines(line_iterable: impl Iterator<Item = impl ToString>) -> ParseLinesResult {
    let mut state = "start_of_file".to_string();
    let mut parses = vec![];
    for (line_idx, line) in line_iterable.enumerate() {
        let prev_state = state.clone();

        //println!("prev_state: {:?} line: {:?}", prev_state, line);
        match parse_line(line.to_string().clone(), &prev_state) {
            Ok((n_state, parsed)) => {
                state = n_state.clone();
                parses.push((n_state, parsed, line.to_string()));
            }
            Err(_err) => {
                return Err(ParseError::LineParseError(line_idx + 1, line.to_string()));
            }
        }
    }
    Ok(parses)
}
