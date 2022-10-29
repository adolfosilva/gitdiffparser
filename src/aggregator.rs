use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug)]
pub struct FileMeta {
    pub no_newline_count: usize,
}

#[derive(Debug, Clone)]
pub enum DiffAction {
    Delete,
    Add,
    Context,
}

impl FromStr for DiffAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "-" => Ok(DiffAction::Delete),
            "+" => Ok(DiffAction::Add),
            " " => Ok(DiffAction::Context),
            e => Err(format!("unknown diff action: {:?}", e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkDiffLine {
    pub from_line_number: usize,
    pub to_line_number: usize,
    pub line: String,
    pub action: DiffAction,
}

#[derive(Debug, Clone)]
pub struct LinePoint {
    pub line_start: usize,
    pub line_count: usize,
}

#[derive(Debug, Clone)]
pub struct ChunkDiff {
    pub from: LinePoint,
    pub to: LinePoint,
    pub lines: Vec<ChunkDiffLine>,
}

#[derive(Debug)]
pub struct ChunkMeta {
    pub from_line_number: usize,
    pub to_line_number: usize,
}

#[derive(Debug)]
pub struct FileDiffPoint {
    pub file: String,
    pub mode: Option<String>,
    pub blob: Option<String>,
    pub end_newline: bool,
}

#[derive(Debug)]
pub struct FileDiff {
    pub from: FileDiffPoint,
    pub to: FileDiffPoint,
    pub is_binary: bool,
    pub chunks: Vec<ChunkDiff>,
}

pub type ParsedLines = Vec<(String, HashMap<String, String>, String)>;

pub fn aggregator(lines: &ParsedLines) -> Vec<FileDiff> {
    let mut file_diff: Option<FileDiff> = None;
    let mut file_meta: Option<FileMeta> = None;
    let mut chunk_diff: Option<ChunkDiff> = None;
    let mut chunk_meta: Option<ChunkMeta> = None;

    let mut file_diffs = vec![];

    for (state, parsed, _) in lines {
        if state == "file_diff_header" {
            if let Some(diff) = file_diff {
                file_diffs.push(diff);

                //file_diff = None;
                //file_meta = None;
                chunk_diff = None;
                chunk_meta = None;
            }

            file_meta = Some(FileMeta {
                no_newline_count: 0,
            });
            file_diff = Some(FileDiff {
                from: FileDiffPoint {
                    file: parsed.get("from_file").unwrap().to_string(),
                    mode: None,
                    blob: None,
                    end_newline: true,
                },
                to: FileDiffPoint {
                    file: parsed.get("to_file").unwrap().to_string(),
                    mode: None,
                    blob: None,
                    end_newline: true,
                },
                is_binary: false,
                chunks: vec![],
            });
            continue;
        }

        if state == "new_file_mode_header" {
            let mode = parsed.get("mode").unwrap().to_string();
            if let Some(ref mut file_diff) = file_diff {
                file_diff.from.mode = Some("0000000".to_string());
                file_diff.to.mode = Some(mode);
            } else {
                unreachable!();
            }
            continue;
        }

        if state == "old_mode_header" {
            let mode = parsed.get("mode").unwrap().to_string();
            if let Some(ref mut file_diff) = file_diff {
                file_diff.from.mode = Some(mode);
            } else {
                unreachable!();
            }
            continue;
        }

        if state == "new_mode_header" {
            let mode = parsed.get("mode").unwrap().to_string();
            if let Some(ref mut file_diff) = file_diff {
                file_diff.to.mode = Some(mode);
            } else {
                unreachable!();
            }
            continue;
        }

        if state == "deleted_file_mode_header" {
            let mode = parsed.get("mode").unwrap().to_string();
            if let Some(ref mut file_diff) = file_diff {
                file_diff.from.mode = Some(mode);
                file_diff.to.mode = Some("0000000".to_string());
            } else {
                unreachable!();
            }
            continue;
        }

        let states = ["a_file_change_header", "b_file_change_header"];
        if states.contains(&state.as_str()) {
            if let Some(ref mut file_diff) = file_diff {
                let file = match state.as_str() {
                    "a_file_change_header" => &file_diff.from.file,
                    "b_file_change_header" => &file_diff.to.file,
                    _ => panic!("unknown state"),
                };

                let f = parsed.get("file");
                if Some(file) != f && f != None {
                    println!("{:?} {:?}", file_diff, parsed);
                    panic!("TODO: Exception text");
                }
            }

            continue;
        }

        if state == "binary_diff" {
            if let Some(ref mut file_diff) = file_diff {
                file_diff.is_binary = true;
            }
            continue;
        }

        if state == "index_diff_header" {
            if let Some(ref mut file_diff) = file_diff {
                let from_blob = parsed.get("from_blob").unwrap().to_string();
                file_diff.from.blob = Some(from_blob);
                let to_blob = parsed.get("to_blob").unwrap().to_string();
                file_diff.to.blob = Some(to_blob);
            } else {
                unreachable!();
            }

            // todo: finish this
            let mode = parsed.get("mode");
            if mode != None {
                if let Some(ref mut file_diff) = file_diff {
                    file_diff.from.mode = Some(mode.unwrap().to_string());
                    file_diff.to.mode = Some(mode.unwrap().to_string());
                }
            }

            continue;
        }

        if state == "chunk_header" {
            let from_line_start = parsed.get("from_line_start").unwrap();
            let to_line_start = parsed.get("to_line_start").unwrap();

            chunk_meta = Some(ChunkMeta {
                from_line_number: from_line_start.parse().unwrap(),
                to_line_number: to_line_start.parse().unwrap(),
            });

            let from_line_count = parsed.get("from_line_count").unwrap();
            let to_line_count = parsed.get("to_line_count").unwrap();
            let diff = ChunkDiff {
                from: LinePoint {
                    line_start: from_line_start.parse().unwrap(),
                    line_count: from_line_count.parse().unwrap(),
                },
                to: LinePoint {
                    line_start: to_line_start.parse().unwrap(),
                    line_count: to_line_count.parse().unwrap(),
                },
                lines: vec![],
            };
            chunk_diff = Some(diff.clone());
            if let Some(ref mut file_diff) = file_diff {
                file_diff.chunks.push(diff);
            } else {
                unreachable!();
            }

            continue;
        }

        if state == "line_diff" {
            if let Some(ref chunk_meta) = chunk_meta {
                let from_line_number = chunk_meta.from_line_number;
                let to_line_number = chunk_meta.to_line_number;
                let a = parsed.get("action").unwrap();
                let action = DiffAction::from_str(a).unwrap();

                let chunk_diff_line = ChunkDiffLine {
                    from_line_number,
                    to_line_number,
                    line: parsed.get("line").unwrap().to_string(),
                    action,
                };

                if let Some(ref mut chunk_diff) = chunk_diff {
                    chunk_diff.lines.push(chunk_diff_line);
                } else {
                    unreachable!();
                }
            } else {
                unreachable!();
            }

            let action = parsed.get("action").unwrap();
            if [" ", "-"].contains(&action.as_str()) {
                if let Some(ref mut chunk_meta) = chunk_meta {
                    chunk_meta.from_line_number += 1;
                }
            }
            if [" ", "+"].contains(&action.as_str()) {
                if let Some(ref mut chunk_meta) = chunk_meta {
                    chunk_meta.to_line_number += 1;
                }
            }

            if let Some(ref file_meta) = file_meta {
                if file_meta.no_newline_count > 0 {
                    if let Some(ref mut file_diff) = file_diff {
                        file_diff.to.end_newline = true;
                        file_diff.from.end_newline = true;
                    }
                }
            }

            continue;
        }

        if state == "no_newline" {
            if let Some(ref mut file_meta) = file_meta {
                file_meta.no_newline_count += 1;
                if file_meta.no_newline_count > 2 {
                    panic!("TODO: Exception text");
                }
            } else {
                unreachable!();
            }
            if let Some(ref mut file_diff) = file_diff {
                file_diff.to.end_newline = false;
            } else {
                unreachable!();
            }
            continue;
        }

        println!("file_diffs: {:?}", file_diffs);
        unreachable!("unexpected {:?} line", state);
    }

    if let Some(file_diff) = file_diff {
        file_diffs.push(file_diff);
    }

    file_diffs
}
